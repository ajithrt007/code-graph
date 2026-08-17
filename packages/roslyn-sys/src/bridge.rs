//! Bridge to the managed Roslyn helper.
//!
//! The helper (`managed/RoslynBridge`) is a small .NET executable that does
//! the actual Roslyn work and prints a JSON graph document to stdout. We
//! keep all Roslyn types inside it; this crate only marshals UTF-8 JSON
//! strings across the process boundary.
//!
//! Strategy: spawn `dotnet <RoslynBridge.dll> <path>` for each analysis and
//! read the JSON from stdout. This avoids in-process CLR hosting and stays
//! portable across macOS / Linux / Windows — the only prerequisite is the
//! .NET 8 SDK, which the solution loader already requires.
//!
//! On machines without `dotnet` (or where the helper has never been built),
//! [`Bridge::init`] reports a clear error and tests can substitute a
//! hand-written fixture via [`Bridge::from_json`].

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};

use crate::dto::AnalysisGraph;

/// Owns the resolved path to the managed helper assembly.
#[derive(Debug, Clone)]
pub struct Bridge {
    /// Absolute path to `RoslynBridge.dll`.
    helper: PathBuf,
}

impl Bridge {
    /// Resolve a usable helper assembly and make sure `dotnet` is on PATH.
    ///
    /// Uses (in order):
    ///   1. `CODEGRAPH_ROSLYN_BRIDGE` — explicit path to `RoslynBridge.dll`
    ///   2. An already-built helper next to this crate:
    ///      `<repo>/managed/RoslynBridge/bin/{Release,Debug}/net8.0/`
    ///   3. A one-off `dotnet build -c Release` of the helper project so the
    ///      vertical slice works on a machine with the .NET SDK installed.
    pub fn init() -> Result<Self> {
        ensure_dotnet().context(
            "`dotnet` was not found on PATH; CodeGraph requires the .NET 8 SDK to analyze C#",
        )?;
        let helper = locate_helper().context(
            "could not locate a built RoslynBridge.dll; try building it with \
             `dotnet build packages/roslyn-sys/managed/RoslynBridge/RoslynBridge.csproj -c Release`",
        )?;
        Ok(Self { helper })
    }

    /// Build a bridge that directly parses an already-produced JSON document.
    /// Used by tests and offline tooling; no `dotnet` required.
    pub fn from_json(_json: &str) -> Result<Self> {
        let helper = PathBuf::from("<test-fixture>");
        Ok(Self { helper })
    }

    /// Path to the managed helper assembly, for diagnostics.
    pub fn managed_assembly(&self) -> &Path {
        &self.helper
    }

    /// Analyze the project/solution at `path` and return the JSON graph
    /// document emitted by the managed helper.
    pub fn analyze_to_json(&self, path: &Path) -> Result<String> {
        let output = Command::new("dotnet")
            .arg(&self.helper)
            .arg(path.as_os_str())
            .output()
            .with_context(|| format!("failed to spawn `dotnet {}`", self.helper.display()))?;

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

fn ensure_dotnet() -> Result<()> {
    let output = Command::new("dotnet")
        .arg("--version")
        .output()
        .context("failed to run `dotnet --version`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("`dotnet --version` failed: {}", stderr.trim()));
    }
    Ok(())
}

/// Absolute path to a built `RoslynBridge.dll`, or `None`.
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

    // The helper project lives next to this crate. `CARGO_MANIFEST_DIR` is
    // baked in at compile time, which keeps the default resolution
    // deterministic on any machine that has the repo checked out.
    let project_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("managed").join("RoslynBridge");
    for config in ["Release", "Debug"] {
        let candidate = project_dir
            .join("bin")
            .join(config)
            .join("net8.0")
            .join("RoslynBridge.dll");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    // Never built in-tree — compile it now with the SDK that's installed.
    build_helper(&project_dir)?;

    let release = project_dir.join("bin").join("Release").join("net8.0").join("RoslynBridge.dll");
    if release.is_file() {
        return Ok(release);
    }
    Err(anyhow!(
        "RoslynBridge build finished but {} was not produced",
        release.display()
    ))
}

fn build_helper(project_dir: &Path) -> Result<()> {
    let csproj = project_dir.join("RoslynBridge.csproj");
    if !csproj.is_file() {
        return Err(anyhow!("helper project not found at {}", csproj.display()));
    }
    let output = Command::new("dotnet")
        .arg("build")
        .arg(&csproj)
        .arg("-c")
        .arg("Release")
        .arg("--nologo")
        .output()
        .with_context(|| format!("failed to run `dotnet build {}`", csproj.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("`dotnet build` failed:\n{}", stderr));
    }
    Ok(())
}