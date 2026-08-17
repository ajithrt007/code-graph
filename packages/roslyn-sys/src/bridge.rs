//! Bridge to the managed Roslyn helper.
//!
//! The helper (`managed/RoslynBridge`) is a small .NET executable that does
//! the actual Roslyn work and prints a JSON graph document to stdout. We
//! keep all Roslyn types inside it; this crate only marshals UTF-8 JSON
//! strings across the process boundary.
//!
//! Strategy: each supported platform ships a **self-contained** bundle of the
//! helper (produced by `scripts/build-roslyn-bridge.sh`), which embeds the
//! .NET runtime. [`Bridge::init`] picks the bundle for the current OS/arch
//! and spawns its native apphost executable directly — no `dotnet` on PATH
//! required at runtime. Tests substitute a hand-written fixture via
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
    ///   2. A prebuilt self-contained bundle next to this crate:
    ///      `<repo>/managed/RoslynBridge/bin/Release/net10.0/<rid>/publish/`
    ///      where `<rid>` matches the current OS/arch.
    pub fn init() -> Result<Self> {
        let helper = locate_helper().context(
            "could not locate a RoslynBridge bundle for this platform; build one once with \
             `./scripts/build-roslyn-bridge.sh` (requires the .NET SDK), or point \
             CODEGRAPH_ROSLYN_BRIDGE at a prebuilt RoslynBridge executable",
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
        let output = Command::new(&self.helper)
            .arg(path.as_os_str())
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

/// Absolute path to a self-contained RoslynBridge executable, or an error.
fn locate_helper() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("CODEGRAPH_ROSLYN_BRIDGE") {
        let p = PathBuf::from(custom);
        if p.is_file() {
            return Ok(p);
        }
        return Err(anyhow!(
            "CODEGRAPH_ROSLYN_BRIDGE set but not a file: {}",
            p.display()
        ));
    }

    let rid = current_rid();
    let executable = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("managed")
        .join("RoslynBridge")
        .join("bin")
        .join("Release")
        .join("net10.0")
        .join(rid)
        .join("publish")
        .join(format!("RoslynBridge{}", std::env::consts::EXE_SUFFIX));
    if executable.is_file() {
        return Ok(executable);
    }

    Err(anyhow!(
        "no RoslynBridge bundle for {} at {}",
        rid,
        executable.display()
    ))
}

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
