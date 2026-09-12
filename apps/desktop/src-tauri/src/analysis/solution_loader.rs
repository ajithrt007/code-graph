//! Loads a .NET solution or project and exposes its compilations.
//!
//! Discovery parses `.sln` and `.csproj` files as plain text. It never
//! shells out to the `dotnet` SDK, so the shipped app works on machines
//! with no .NET toolchain installed: the Roslyn analysis itself runs in
//! the self-contained RoslynBridge bundle (which embeds the runtime), and
//! everything this loader needs — project list, target framework — is
//! readable directly from the project files.
//!
//! This also keeps "open project" fast and, on Windows, free of flashing
//! `dotnet.exe` console windows (one per spawn without CREATE_NO_WINDOW).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

/// Lightweight description of a project discovered inside a solution or
/// standalone on disk. We only need the path and target framework to feed
/// Roslyn; richer MSBuild parsing is deliberately deferred.
#[derive(Debug, Clone)]
pub struct ProjectDescriptor {
    pub project_path: PathBuf,
    pub target_framework: Option<String>,
}

/// Loads .NET solutions and projects by reading their files directly —
/// no `dotnet` SDK required.
#[derive(Debug, Default, Clone)]
pub struct SolutionLoader;

impl SolutionLoader {
    pub fn new() -> Self {
        Self
    }

    /// Resolve a user-provided path to one or more `.csproj` projects.
    ///
    /// Accepts either a `.sln` (projects parsed out of the solution file)
    /// or a `.csproj` directly. The target framework is read from the
    /// `.csproj` XML; if unavailable, `None` is returned.
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
                return Ok(vec![self.describe_project(&p)]);
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
            Some("csproj") => Ok(vec![self.describe_project(&canonical)]),
            other => Err(anyhow!(
                "unsupported input `{}` (expected .sln, .csproj, or a directory)",
                other.unwrap_or("<no extension>")
            )),
        }
    }

    fn discover_from_sln(&self, sln: &Path) -> Result<Vec<ProjectDescriptor>> {
        // Parse the .sln text directly instead of `dotnet sln list`.
        // Project lines look like:
        //   Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "App", "src\App.csproj", "{GUID}"
        // Solution folders and non-C# projects never reference a `.csproj`,
        // so filtering on that extension skips them naturally.
        let text = std::fs::read_to_string(sln)
            .with_context(|| format!("failed to read solution {}", sln.display()))?;
        let sln_dir = sln.parent().unwrap_or_else(|| Path::new("."));

        let mut projects = Vec::new();
        for raw in text.lines() {
            let Some(path_str) = sln_project_path(raw) else {
                continue;
            };
            // Solution files use Windows separators even when read on
            // Unix; normalize so relative joins work everywhere.
            let relative = path_str.replace('\\', "/");
            let path = if Path::new(&relative).is_absolute() {
                PathBuf::from(relative)
            } else {
                sln_dir.join(relative)
            };
            projects.push(self.describe_project(&path));
        }

        if projects.is_empty() {
            return Err(anyhow!(
                "no .csproj projects found in solution {}",
                sln.display()
            ));
        }
        Ok(projects)
    }

    fn describe_project(&self, csproj: &Path) -> ProjectDescriptor {
        ProjectDescriptor {
            project_path: csproj.to_path_buf(),
            target_framework: read_target_framework(csproj),
        }
    }
}

/// Extract the `.csproj` path from a `.sln` `Project(...) = ...` line, or
/// `None` for any other line (headers, solution folders, C++ projects).
fn sln_project_path(line: &str) -> Option<&str> {
    let line = line.trim();
    if !line.starts_with("Project(") {
        return None;
    }
    let rhs = line.split_once('=')?.1;
    // Quoted segments: name, path, guid. The path is the first segment
    // ending in `.csproj`.
    let mut segments = Vec::new();
    let mut rest = rhs;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else {
            break;
        };
        segments.push(after[..end].trim());
        rest = &after[end + 1..];
    }
    segments.into_iter().find(|segment| {
        segment.len() > ".csproj".len()
            && segment
                .get(segment.len() - ".csproj".len()..)
                .is_some_and(|ext| ext.eq_ignore_ascii_case(".csproj"))
    })
}

/// Read `<TargetFramework>` (or the first entry of `<TargetFrameworks>`)
/// from a `.csproj` file. Returns `None` when the file can't be read or
/// declares no framework — callers treat this as informational only.
fn read_target_framework(csproj: &Path) -> Option<String> {
    let text = std::fs::read_to_string(csproj).ok()?;
    let value = tag_value(&text, "TargetFramework")
        .or_else(|| tag_value(&text, "TargetFrameworks"))?;
    let first = value.split(';').next()?.trim();
    if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    }
}

/// Contents of a simple `<Tag>value</Tag>` element. MSBuild tags are
/// case-sensitive and never carry attributes for these two names, so an
/// exact scan is sufficient — no XML parser needed.
fn tag_value(text: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = text.find(open.as_str())? + open.len();
    let rest = text.get(start..)?;
    let end = rest.find(close.as_str())?;
    let value = rest.get(..end)?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "codegraph-loader-test-{}-{}-{}",
            std::process::id(),
            tag,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock before epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp test dir");
        // Canonicalize: temp roots are symlinked on some OSes (macOS
        // /var -> /private/var) and discovery canonicalizes inputs.
        dir.canonicalize().expect("canonicalize temp test dir")
    }

    fn write(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().expect("test file has parent"))
            .expect("create test parent");
        std::fs::write(path, contents).expect("write test file");
    }

    const SLN: &str = r#"
Microsoft Visual Studio Solution File, Format Version 12.00
Project("{9A19103F-16F7-4668-BE54-9A1E7A4F7556}") = "App", "src\App.csproj", "{11111111-1111-1111-1111-111111111111}"
EndProject
Project("{9A19103F-16F7-4668-BE54-9A1E7A4F7556}") = "Lib", "Lib\Lib.csproj", "{22222222-2222-2222-2222-222222222222}"
EndProject
Project("{2150E333-8FDC-42A3-9474-1A3956D46DE8}") = "Solution Items", "Solution Items", "{33333333-3333-3333-3333-333333333333}"
EndProject
Project("{8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942}") = "Native", "Native\Native.vcxproj", "{44444444-4444-4444-4444-444444444444}"
EndProject
Global
EndGlobal
"#;

    #[test]
    fn sln_parsing_finds_csproj_projects_only() {
        let dir = unique_dir("sln");
        let sln = dir.join("App.sln");
        write(&sln, SLN);
        write(&dir.join("src").join("App.csproj"), "<Project></Project>");
        write(&dir.join("Lib").join("Lib.csproj"), "<Project></Project>");

        let loader = SolutionLoader::new();
        let projects = loader.discover_projects(&sln).expect("discover");
        let mut paths: Vec<_> = projects
            .iter()
            .map(|p| p.project_path.clone())
            .collect();
        paths.sort();
        assert_eq!(
            paths,
            vec![dir.join("Lib").join("Lib.csproj"), dir.join("src").join("App.csproj")],
            "expected the two .csproj entries (folders/vcxproj skipped)"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn sln_without_csproj_is_an_error() {
        let dir = unique_dir("sln-empty");
        let sln = dir.join("Empty.sln");
        write(&sln, "Microsoft Visual Studio Solution File, Format Version 12.00\n");

        let err = SolutionLoader::new()
            .discover_projects(&sln)
            .expect_err("should fail");
        assert!(
            err.to_string().contains("no .csproj projects found"),
            "unexpected error: {err}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn target_framework_prefers_single_over_list() {
        let dir = unique_dir("tfm");
        let single = dir.join("Single.csproj");
        write(
            &single,
            "<Project><PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup></Project>",
        );
        let multi = dir.join("Multi.csproj");
        write(
            &multi,
            "<Project><PropertyGroup><TargetFrameworks>net8.0;net10.0</TargetFrameworks></PropertyGroup></Project>",
        );
        let none = dir.join("None.csproj");
        write(&none, "<Project></Project>");

        assert_eq!(
            read_target_framework(&single).as_deref(),
            Some("net10.0")
        );
        assert_eq!(
            read_target_framework(&multi).as_deref(),
            Some("net8.0"),
            "multi-targeted takes the first entry"
        );
        assert_eq!(read_target_framework(&none), None);
        assert_eq!(
            read_target_framework(&dir.join("Missing.csproj")),
            None,
            "unreadable file degrades to None"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn directory_prefers_sln_over_csproj() {
        let dir = unique_dir("dir");
        write(&dir.join("Only.csproj"), "<Project></Project>");
        write(
            &dir.join("Via.csproj"),
            "<Project><PropertyGroup><TargetFramework>net10.0</TargetFramework></PropertyGroup></Project>",
        );
        write(
            &dir.join("All.sln"),
            "Project(\"{9A19103F-16F7-4668-BE54-9A1E7A4F7556}\") = \"Via\", \"Via.csproj\", \"{55555555-5555-5555-5555-555555555555}\"\nEndProject\n",
        );

        let projects = SolutionLoader::new()
            .discover_projects(&dir)
            .expect("discover");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].project_path, dir.join("Via.csproj"));
        assert_eq!(
            projects[0].target_framework.as_deref(),
            Some("net10.0")
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
