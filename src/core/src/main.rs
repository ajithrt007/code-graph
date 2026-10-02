use std::{error, path::{Path, PathBuf}};

use code_graph_core::{analyzer::Analyze, language_resolvers::language_resolver, supported_languages::SupportedLanguages};

#[tokio::main]
async fn main() {
    let path_str: &str = "/Users/ajithrt/Work/code-graph/examples/OrderSystem";

    let path = ensure_path(path_str).unwrap();

    let language = language_resolver(&path).unwrap();
    
    ensure_current_lang_analyzer_exists(language).await().unwrap();

    init_index_async(&path, language).await().unwrap();
}

fn ensure_path(path_str: &str) -> Result<PathBuf, String> {
    let path = Path::new(path_str); 
    
    if !path.is_absolute() {
        Err(format!("Path '{}' is not an absolute path", path.display()));
    }

    if !path.is_dir() {
        Err(format!("Path '{}' is not a directory", path.display()));
    }

    Ok(path.to_path_buf());
}

async fn ensure_current_lang_analyzer_exists_async(language: SupportedLanguages) -> Result<(), String>{
    // if !language.is_analyzer_available(){
    //     Err("language specific analyzer not present.".to_string());
    // }

    Ok(())
}

async fn init_index_async(path: &Path, language: SupportedLanguages) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1000);

    tokio::spawn(async move {
        language.index_async(&path, tx).await.unwrap();
    });

    while let Some(number) = rx.recv().await {
        println!("Project loading...: {} %", number);
    }

    println!("Project ready.");
}