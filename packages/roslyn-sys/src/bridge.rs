//! Bridge to the managed Roslyn helper.
//!
//! The helper (`managed/RoslynBridge`) is a small .NET executable that does
//! the actual Roslyn work and prints a JSON graph document to stdout. We
//! keep all Roslyn types inside it; this crate only marshals UTF-8 JSON
//! strings across the process boundary.
//!
//! Strategy: each supported platform ships a **self-contained** bundle of the
//! helper (produced by `scripts/build-roslyn-bridge.sh`), which embeds the
//! .NET runtime. The desktop app embeds that bundle in its installer via
//! Tauri `bundle.resources` (staged by `scripts/collect-roslyn-bridge.sh`)
//! and passes its runtime resource directory to [`Bridge::init_with_resource_dir`].
//! [`Bridge::init`] keeps working for dev checkouts by falling back to the
//! publish directory in the source tree — no `dotnet` on PATH required at
//! runtime either way. Tests substitute a hand-written fixture via
//! [`Bridge::from_json`].

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};

use crate::dto::AnalysisGraph;

/// Owns the resolved path to the managed helper's native executable.
#[derive(Debug, Clone)]
pub struct Bridge {
    /// Absolute path to the self-contained helper executable.
    helper: PathBuf,
}

impl Bridge {
    /// Resolve the helper bundle for the current OS/arch.
    ///
    /// Uses (in order):
    ///   1. `CODEGRAPH_ROSLYN_BRIDGE` — explicit path to the helper executable
    ///      (when it points at an existing file; a stale value falls through
    ///      to the search below instead of failing).
    ///   2. A prebuilt self-contained bundle next to this crate:
    ///      `<repo>/managed/RoslynBridge/bin/Release/net10.0/<rid>/publish/`
    ///      where `<rid>` matches the current OS/arch.
    pub fn init() -> Result<Self> {
        let helper = locate_helper(None).context(
            "could not locate a RoslynBridge bundle for this platform; build one once with \
             `./scripts/build-roslyn-bridge.sh` (requires the .NET SDK), or point \
             CODEGRAPH_ROSLYN_BRIDGE at a prebuilt RoslynBridge executable",
        )?;
        Ok(Self { helper })
    }

    /// Resolve the helper bundle, preferring a copy embedded in the installed
    /// app's resource directory (see `bundle.resources` in `tauri.conf.json`).
    ///
    /// `resource_dir` is the runtime value of Tauri's `app.path().resource_dir()`;
    /// this crate stays Tauri-free by taking it as a plain path. Resolution
    /// order:
    ///   1. `CODEGRAPH_ROSLYN_BRIDGE` — explicit override, when it points at
    ///      an existing file. A stale/missing override is *not* fatal: it is
    ///      recorded in the error report and the search continues below, so a
    ///      leftover env var on one machine can't brick the installed app.
    ///   2. `<resource_dir>/resources/roslyn-bridge/` (installed layout;
    ///      `bundle.resources` preserves the relative structure).
    ///   3. `<resource_dir>/roslyn-bridge/` (alternate flattened layout).
    ///   4. `<resource_dir>/RoslynBridge[.exe]` (helper repackaged at the base root).
    ///   5. The same three layouts relative to the running executable's own
    ///      directory (`std::env::current_exe`; covers installs where
    ///      `resource_dir()` misses the bundle root — observed on Windows,
    ///      where both usually equal the install root).
    ///   6. The dev-tree publish directory (same as [`Bridge::init`]).
    ///
    /// A miss returns an error listing the env override, every path searched,
    /// and the runtime `resource_dir` / `current_exe` values, so a miss is
    /// diagnosable from the message alone.
    pub fn init_with_resource_dir(resource_dir: &Path) -> Result<Self> {
        let helper = locate_helper(Some(resource_dir)).context(
            "could not locate a RoslynBridge bundle for this platform; reinstall the app \
             (the helper ships inside the installer), or point CODEGRAPH_ROSLYN_BRIDGE \
             at a prebuilt RoslynBridge executable",
        )?;
        Ok(Self { helper })
    }

    /// Build a bridge that directly parses an already-produced JSON document.
    /// Used by tests and offline tooling; no bundle required.
    pub fn from_json(_json: &str) -> Result<Self> {
        let helper = PathBuf::from("<test-fixture>");
        Ok(Self { helper })
    }

    /// Path to the helper executable, for diagnostics.
    pub fn helper_path(&self) -> &Path {
        &self.helper
    }

    /// Analyze the project/solution at `path` and return the JSON graph
    /// document emitted by the managed helper.
    pub fn analyze_to_json(&self, path: &Path) -> Result<String> {
        let mut command = Command::new(&self.helper);
        command.arg(path.as_os_str());
        // On Windows, console-subsystem children pop a visible console
        // window per spawn; suppress it — stdout/stderr stay piped.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW
            command.creation_flags(0x08000000);
        }
        let output = command
            .output()
            .with_context(|| format!("failed to spawn `{}`", self.helper.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(anyhow!(
                "RoslynBridge exited with {}:\n{}\n{}",
                output.status,
                stdout,
                stderr
            ));
        }

        // The helper prints exactly one JSON document to stdout.
        let json = String::from_utf8(output.stdout)
            .context("RoslynBridge emitted non-UTF-8 output")?
            .trim()
            .to_string();
        if json.is_empty() {
            return Err(anyhow!("RoslynBridge returned no output"));
        }
        Ok(json)
    }

    /// Convert a JSON document produced by the managed helper into a Rust
    /// [`AnalysisGraph`] value (the same shape the analyzer consumes).
    pub fn parse_graph(&self, json: &str) -> Result<AnalysisGraph> {
        Ok(serde_json::from_str(json)?)
    }
}

/// Absolute path to a self-contained RoslynBridge executable, or an error
/// listing every location searched (so a miss is diagnosable from the
/// message alone).
fn locate_helper(resource_dir: Option<&Path>) -> Result<PathBuf> {
    let env_override = std::env::var("CODEGRAPH_ROSLYN_BRIDGE")
        .ok()
        .map(PathBuf::from);
    let current_exe = std::env::current_exe().ok();
    locate_helper_with(
        resource_dir,
        env_override.as_deref(),
        current_exe.as_deref(),
        Some(&dev_bundle_path()),
    )
}

/// Testable core of [`locate_helper`]: the env override, the running
/// executable, and the dev-tree fallback are parameters so tests don't touch
/// process-global state (pass `None`/nonexistent for the fallback to force
/// the miss branch).
fn locate_helper_with(
    resource_dir: Option<&Path>,
    env_override: Option<&Path>,
    current_exe: Option<&Path>,
    dev_fallback: Option<&Path>,
) -> Result<PathBuf> {
    let mut notes: Vec<String> = Vec::new();

    // Explicit override wins when it exists, but a stale value must not brick
    // the app while a perfectly good bundled helper sits next to it: record
    // the miss and keep searching. (Seen on Windows, where a leftover
    // CODEGRAPH_ROSLYN_BRIDGE from an earlier troubleshooting session made
    // every launch fail at init even though the installer bundles the
    // helper.)
    if let Some(custom) = env_override {
        if custom.is_file() {
            return Ok(custom.to_path_buf());
        }
        notes.push(format!(
            "CODEGRAPH_ROSLYN_BRIDGE={} is not a file; continuing search",
            custom.display()
        ));
    }

    // Candidate base dirs: Tauri's resource dir first, then the running
    // executable's own directory. The latter covers installs where
    // `resource_dir()` doesn't point at the bundle root (observed on
    // Windows: the helper sits at
    // `<install>/resources/roslyn-bridge/RoslynBridge.exe` next to the
    // running exe, yet resource-dir resolution missed it).
    let mut bases: Vec<PathBuf> = Vec::new();
    if let Some(dir) = resource_dir {
        bases.push(dir.to_path_buf());
    }
    if let Some(exe) = current_exe {
        if let Some(dir) = exe.parent() {
            if !bases.iter().any(|b| b == dir) {
                bases.push(dir.to_path_buf());
            }
        }
    }

    let mut searched = Vec::new();
    for base in &bases {
        for candidate in bundled_candidates(base) {
            if candidate.is_file() {
                ensure_executable(&candidate);
                return Ok(candidate);
            }
            searched.push(candidate);
        }
    }

    if let Some(executable) = dev_fallback {
        if executable.is_file() {
            return Ok(executable.to_path_buf());
        }
    }

    let mut report = format!("no RoslynBridge bundle for {}", current_rid());
    report.push_str(&format!(
        " (resource_dir={}; current_exe={})",
        resource_dir
            .map(|d| d.display().to_string())
            .as_deref()
            .unwrap_or("<unavailable>"),
        current_exe
            .map(|e| e.display().to_string())
            .as_deref()
            .unwrap_or("<unavailable>"),
    ));
    for note in &notes {
        report.push_str(&format!("\n  note: {note}"));
    }
    if searched.is_empty() {
        report.push_str("; no resource/executable directories available to search");
    } else {
        report.push_str("; searched:");
        for path in &searched {
            report.push_str(&format!("\n  - {}", path.display()));
        }
    }
    report.push_str(&format!(
        "\n  (dev fallback: {})",
        dev_fallback
            .map(|d| d.display().to_string())
            .as_deref()
            .unwrap_or("<disabled>")
    ));
    Err(anyhow!(report))
}

/// Candidate locations inside an installed app's resource directory.
///
/// Tauri copies `bundle.resources` entries preserving their relative
/// structure, so `resources/roslyn-bridge/` in `src-tauri` lands at
/// `<resource_dir>/resources/roslyn-bridge/`. The flattened and root-level
/// variants are probed too so a future repackaging doesn't silently break
/// analysis.
fn bundled_candidates(resource_dir: &Path) -> [PathBuf; 3] {
    let exe = format!("RoslynBridge{}", std::env::consts::EXE_SUFFIX);
    [
        resource_dir
            .join("resources")
            .join("roslyn-bridge")
            .join(&exe),
        resource_dir.join("roslyn-bridge").join(&exe),
        resource_dir.join(&exe),
    ]
}

/// Publish directory used during development (repo checkout present).
fn dev_bundle_path() -> PathBuf {
    let rid = current_rid();
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("managed")
        .join("RoslynBridge")
        .join("bin")
        .join("Release")
        .join("net10.0")
        .join(rid)
        .join("publish")
        .join(format!("RoslynBridge{}", std::env::consts::EXE_SUFFIX))
}

/// Best-effort `chmod +x`: bundlers/zips can strip the apphost's exec bit.
#[cfg(unix)]
fn ensure_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut permissions = metadata.permissions();
        let mode = permissions.mode();
        if mode & 0o111 == 0 {
            permissions.set_mode(mode | 0o755);
            let _ = std::fs::set_permissions(path, permissions);
        }
    }
}

#[cfg(not(unix))]
fn ensure_executable(_path: &Path) {}

/// Runtime identifier of the current OS/arch. Matches the bundle directories
/// produced by `scripts/build-roslyn-bridge.sh`.
fn current_rid() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "osx-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "osx-x64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "win-x64"
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        "win-arm64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-arm64"
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
    )))]
    {
        compile_error!("unsupported platform: no RoslynBridge bundle RID for this OS/arch");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe_name() -> String {
        format!("RoslynBridge{}", std::env::consts::EXE_SUFFIX)
    }

    fn unique_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "codegraph-bridge-test-{}-{}-{}",
            std::process::id(),
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        dir
    }

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().expect("candidate has parent"))
            .expect("create candidate parent");
        std::fs::write(path, b"fake-helper").expect("write fake helper");
    }

    #[test]
    fn installed_bundled_layout_is_found() {
        let resource_dir = unique_dir("bundled");
        let helper = resource_dir
            .join("resources")
            .join("roslyn-bridge")
            .join(exe_name());
        touch(&helper);

        let bridge = Bridge::init_with_resource_dir(&resource_dir).expect("resolve bundled helper");
        assert_eq!(bridge.helper_path(), helper);
        std::fs::remove_dir_all(&resource_dir).ok();
    }

    #[test]
    fn flattened_bundled_layout_is_found() {
        let resource_dir = unique_dir("flattened");
        let helper = resource_dir.join("roslyn-bridge").join(exe_name());
        touch(&helper);

        let bridge =
            Bridge::init_with_resource_dir(&resource_dir).expect("resolve flattened helper");
        assert_eq!(bridge.helper_path(), helper);
        std::fs::remove_dir_all(&resource_dir).ok();
    }

    #[test]
    fn root_level_helper_is_found() {
        // Repackaged installs may stage the helper at the base root.
        let base = unique_dir("root-level");
        let helper = base.join(exe_name());
        touch(&helper);

        let resolved =
            locate_helper_with(Some(&base), None, None, None).expect("resolve root-level helper");
        assert_eq!(resolved, helper);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn stale_env_override_falls_back_to_bundled() {
        // A leftover CODEGRAPH_ROSLYN_BRIDGE pointing nowhere must not brick
        // the app while a bundled helper exists (the Windows init failure).
        let resource_dir = unique_dir("stale-env");
        let helper = resource_dir
            .join("resources")
            .join("roslyn-bridge")
            .join(exe_name());
        touch(&helper);
        let stale = resource_dir.join("gone").join(exe_name());

        let resolved = locate_helper_with(Some(&resource_dir), Some(&stale), None, None)
            .expect("stale override falls back to bundled helper");
        assert_eq!(resolved, helper);
        std::fs::remove_dir_all(&resource_dir).ok();
    }

    #[test]
    fn valid_env_override_still_wins() {
        let dir = unique_dir("env-wins");
        let from_env = dir.join(format!("custom-{}", exe_name()));
        let from_bundle = dir.join("resources").join("roslyn-bridge").join(exe_name());
        touch(&from_env);
        touch(&from_bundle);

        let resolved = locate_helper_with(Some(&dir), Some(&from_env), None, None)
            .expect("valid override wins");
        assert_eq!(resolved, from_env);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn miss_error_names_override_bases_and_candidates() {
        let resource_dir = unique_dir("miss");
        let stale = resource_dir.join("stale").join(exe_name());
        let fake_exe = resource_dir.join("bin").join("app.exe");

        let missing_dev = resource_dir.join("dev-fallback").join("RoslynBridge");
        let err = locate_helper_with(Some(&resource_dir), Some(&stale), Some(&fake_exe), Some(&missing_dev))
            .expect_err("should fail with nothing staged");
        let message = format!("{err:#}");
        // The stale override is recorded, not silently ignored…
        assert!(
            message.contains("CODEGRAPH_ROSLYN_BRIDGE"),
            "missing env note: {message}"
        );
        assert!(
            message.contains(&stale.display().to_string()),
            "missing stale path: {message}"
        );
        // …and the report names the runtime bases plus every candidate, so
        // the next failure is diagnosable from the popup alone.
        assert!(
            message.contains(&resource_dir.display().to_string()),
            "missing resource_dir: {message}"
        );
        assert!(
            message.contains(&fake_exe.display().to_string()),
            "missing current_exe: {message}"
        );
        assert!(
            message.contains("resources"),
            "missing searched candidates: {message}"
        );
        assert!(
            message.contains(&missing_dev.display().to_string()),
            "missing dev fallback: {message}"
        );
        std::fs::remove_dir_all(&resource_dir).ok();
    }

    #[test]
    fn env_override_wins_over_resource_dir() {
        let dir = unique_dir("env");
        let from_env = dir.join(format!("custom-{}", exe_name()));
        let from_bundle = dir.join("resources").join("roslyn-bridge").join(exe_name());
        touch(&from_env);
        touch(&from_bundle);

        let previous = std::env::var_os("CODEGRAPH_ROSLYN_BRIDGE");
        std::env::set_var("CODEGRAPH_ROSLYN_BRIDGE", &from_env);
        let resolved = Bridge::init_with_resource_dir(&dir).expect("resolve with env override");
        match previous {
            Some(value) => std::env::set_var("CODEGRAPH_ROSLYN_BRIDGE", value),
            None => std::env::remove_var("CODEGRAPH_ROSLYN_BRIDGE"),
        }

        assert_eq!(resolved.helper_path(), from_env);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// End-to-end oracle over `examples/OrderSystem` through the installed-app
    /// layout (`src-tauri/` standing in for Tauri's `$RESOURCE` dir).
    ///
    /// Ignored by default: needs the real bundle published AND staged
    /// (`build-roslyn-bridge.sh` + `collect-roslyn-bridge.sh`). Run with
    /// `cargo test -p roslyn-sys -- --ignored`.
    #[test]
    #[ignore]
    fn analyzes_order_system_from_staged_resources() {
        let resource_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("apps")
            .join("desktop")
            .join("src-tauri");
        let bridge = Bridge::init_with_resource_dir(&resource_dir)
            .expect("resolve staged RoslynBridge bundle");
        assert!(
            bridge.helper_path().ends_with(
                Path::new("resources")
                    .join("roslyn-bridge")
                    .join(exe_name())
            ),
            "unexpected helper path: {}",
            bridge.helper_path().display()
        );

        let target = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/OrderSystem/OrderSystem.csproj")
            .canonicalize()
            .expect("OrderSystem example exists");
        let json = bridge
            .analyze_to_json(&target)
            .expect("analyze OrderSystem");
        let graph = bridge
            .parse_graph(&json)
            .expect("parse graph JSON")
            .into_domain()
            .expect("convert to domain graph");

        assert_eq!(graph.methods.len(), 9, "expected 9 methods");
        assert_eq!(graph.edges.len(), 7, "expected 7 edges");
    }
}
