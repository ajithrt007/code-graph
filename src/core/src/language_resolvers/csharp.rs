use walkdir::WalkDir;
use std::path::Path;

use crate::language_resolvers::LanguageResolver;
use crate::supported_languages::SupportedLanguages;

pub struct CSharpProjectAnalyzer;

impl LanguageResolver for CSharpProjectAnalyzer {
    fn is_project(&self, path: &Path) -> bool {
        contains_dotnet_files(path)
    }

    fn get_language(&self) -> SupportedLanguages {
        SupportedLanguages::CSharp
    }
}

fn contains_dotnet_files(dir: &Path) -> bool {
        WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| {
                e.path().extension().map_or(false, |ext| {
                    ext.eq_ignore_ascii_case("csproj") || ext.eq_ignore_ascii_case("sln")
                })
            })
    }