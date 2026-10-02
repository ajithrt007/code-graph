use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};

use crate::supported_languages::SupportedLanguages;




impl Analyze for SupportedLanguages {
    async  fn start_external_analyzer_async(&self) -> impl Future<Output = Result<, Box<dyn std::error::Error>>> {
        
    }

    async fn stop_external_analyzer_async(&self) -> Result<(), Box<dyn std::error::Error>> {
        
    }

    async fn is_analyzer_available(&self) -> bool {
        match &self {
            SupportedLanguages::CSharp => {

            },
        }
    }

    async fn index(&self, path: &Path) -> Result<(), String> {
        match &self {
            SupportedLanguages::CSharp => {

            },
        }
    }

    async fn index_file(&self, paths: Vec<&Path>) -> Result<Vec<&str>, String> {
        match &self {
            SupportedLanguages::CSharp => {

            },
        }
    }
}


async fn start(
    executable: &str,
) -> Result<Self, Box<dyn std::error::Error>> {
    
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    
    // Continuously read analyzer output.
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        
        while let Ok(Some(line)) = lines.next_line().await {
            println!("Analyzer: {line}");
            
            // Later:
            // parse event
            // save to SQLite
            // publish to EventBus
        }
    });
    
    Ok()
}

async fn analyze(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    self.stdin
    .write_all(b"{\"method\":\"analyze\"}\n")
    .await?;
    
    self.stdin.flush().await?;
    
    Ok(())
}