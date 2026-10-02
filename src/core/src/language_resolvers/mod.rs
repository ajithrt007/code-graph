use std::path::Path;
use crate::supported_languages::SupportedLanguages;

mod csharp;

pub trait LanguageResolver {
    fn is_project(&self, path: &Path) -> bool;
    fn get_language(&self) -> SupportedLanguages;
}

fn get_language_resolvers() -> Vec<Box<dyn LanguageResolver>> {
    Vec::from([
        Box::new(csharp::CSharpProjectAnalyzer) as Box<dyn LanguageResolver>,
    ])
}

pub fn language_resolver(path: &Path) -> Result<SupportedLanguages, String> {
    let results:Vec<SupportedLanguages> = get_language_resolvers()
            .iter()
            .filter(|resolver| resolver.is_project(path))
            .map(|resolver| resolver.get_language())
            .collect();

        match results.len() {
            0 => Err("No project.".to_string()),
            1 =>  Ok(results[0]),
            _ => Err("Codegraph doesn't support multiple language projects.".to_string()),
        }
}