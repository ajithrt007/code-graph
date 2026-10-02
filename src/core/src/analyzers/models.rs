

pub struct AnalyzerClient {
    stdin: ChildStdin,
    child: Child,
}



pub trait Analyze {
    async fn start_external_analyzer_async(&self) -> Result<, Box<dyn std::error::Error>>;
    async fn stop_external_analyzer_async(&self) -> Result<(), Box<dyn std::error::Error>>;
    async fn is_analyzer_available(&self) -> bool;
    async fn index(&self, path: &Path, tx: mpsc::Sender<i32>) -> Result<(), String>; //streams project init to core -> client
    async fn index_file(&self, paths: Vec<&Path>) -> Result<Vec<&str>, String>;
}