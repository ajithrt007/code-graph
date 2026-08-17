//! Loads a .NET solution or project and exposes its compilations.
//!
//! This is intentionally a thin wrapper around `dotnet`/`Roslyn` so the rest
//! of the analyzer can work purely against `Microsoft.CodeAnalysis`
//! types. All Roslyn types stay inside this module and its callers in the
//! `analysis` submodule; they do not escape into the application layer.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};

/// Lightweight description of a project discovered inside a solution or
/// standalone on disk. We only need the path and target framework to feed
/// Roslyn; richer MSBuild parsing is deliberately deferred.
#[derive(Debug, Clone)]
pub struct ProjectDescriptor {
    pub project_path: PathBuf,
    pub target_framework: Option<String>,
}

/// Loads .NET solutions and projects via `dotnet`.
#[derive(Debug, Default, Clone)]
pub struct SolutionLoader;

impl SolutionLoader {
    pub fn new() -> Self {
        Self
    }

    /// Resolve a user-provided path to one or more `.csproj` projects.
    ///
    /// Accepts either a `.sln` (expanded via `dotnet sln list`) or a
    /// `.csproj` directly. The target framework is queried from
    /// `dotnet build` properties; if unavailable, `None` is returned.
    pub fn discover_projects(&self, input: &Path) -> Result<Vec<ProjectDescriptor>> {
        if !input.exists() {
            return Err(anyhow!("path does not exist: {}", input.display()));
        }

        let canonical = input.canonicalize().unwrap_or_else(|_| input.to_path_buf());

        if canonical.is_dir() {
            // Look for a single .csproj or .sln inside the directory.
            let mut sln: Option<PathBuf> = None;
            let mut csproj: Option<PathBuf> = None;
            for entry in std::fs::read_dir(&canonical)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    match ext.to_ascii_lowercase().as_str() {
                        "sln" => sln = Some(path),
                        "csproj" => csproj = Some(path),
                        _ => {}
                    }
                }
            }
            if let Some(s) = sln {
                return self.discover_from_sln(&s);
            }
            if let Some(p) = csproj {
                return Ok(vec![self.describe_project(&p)?]);
            }
            return Err(anyhow!(
                "no .sln or .csproj found in directory {}",
                canonical.display()
            ));
        }

        match canonical
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("sln") => self.discover_from_sln(&canonical),
            Some("csproj") => Ok(vec![self.describe_project(&canonical)?]),
            other => Err(anyhow!(
                "unsupported input `{}` (expected .sln, .csproj, or a directory)",
                other.unwrap_or("<no extension>")
            )),
        }
    }

    fn discover_from_sln(&self, sln: &Path) -> Result<Vec<ProjectDescriptor>> {
        // Use `dotnet sln list` to enumerate projects. This is the most
        // portable approach and avoids re-implementing the .sln parser.
        let output = Command::new("dotnet")
            .arg("sln")
            .arg(sln)
            .arg("list")
            .output()
            .with_context(|| format!("failed to run `dotnet sln list` on {}", sln.display()))?;

        if !output.status.success() {
            return Err(anyhow!(
                "`dotnet sln list` failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sln_dir = sln.parent().unwrap_or_else(|| Path::new("."));
        let mut projects = Vec::new();
        for raw in stdout.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            // Skip header lines like "Project(s)" and "----------".
            if line.contains("---") || line.eq_ignore_ascii_case("project(s)") {
                continue;
            }
            // Tokens are space-separated; the path is the last token (or the
            // only token for absolute paths).
            let path_str = line.split_whitespace().last().unwrap_or(line);
            let path = if Path::new(path_str).is_absolute() {
                PathBuf::from(path_str)
            } else {
                sln_dir.join(path_str)
            };
            if path.extension().and_then(|e| e.to_str()) == Some("csproj") {
                projects.push(self.describe_project(&path)?);
            }
        }

        if projects.is_empty() {
            return Err(anyhow!(
                "no .csproj projects found in solution {}",
                sln.display()
            ));
        }
        Ok(projects)
    }

    fn describe_project(&self, csproj: &Path) -> Result<ProjectDescriptor> {
        let tfm = self.query_target_framework(csproj).ok();
        Ok(ProjectDescriptor {
            project_path: csproj.to_path_buf(),
            target_framework: tfm,
        })
    }

    fn query_target_framework(&self, csproj: &Path) -> Result<String> {
        // `dotnet msbuild -getProperty:TargetFramework` is supported by
        // modern SDKs. Fall back to `TargetFrameworks` if multi-targeted.
        let try_prop = |name: &str| -> Result<String> {
            let output = Command::new("dotnet")
                .arg("msbuild")
                .arg(csproj)
                .arg(format!("-getProperty:{}", name))
                .output()
                .with_context(|| {
                    format!("failed to run `dotnet msbuild -getProperty:{}`", name)
                })?;
            if !output.status.success() {
                return Err(anyhow!(
                    "msbuild -getProperty:{} failed: {}",
                    name,
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if s.is_empty() {
                Err(anyhow!("empty property"))
            } else {
                Ok(s)
            }
        };

        if let Ok(v) = try_prop("TargetFramework") {
            // Multi-targeted projects may report a semicolon-separated list
            // under `TargetFrameworks`.
            if let Some(first) = v.split(';').next() {
                return Ok(first.trim().to_string());
            }
        }
        try_prop("TargetFrameworks").and_then(|v| {
            v.split(';')
                .next()
                .map(|s| s.trim().to_string())
                .ok_or_else(|| anyhow!("no target framework"))
        })
    }
}
