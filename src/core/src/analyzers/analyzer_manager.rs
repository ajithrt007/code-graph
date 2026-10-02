

pub struct AnalyzerManager {
    analyzers: HashMap<SupportedLanguages, Box<dyn Analyze>,
    live_analyzers: HashMap<SupportedLanguages, AnalyzerClient>,
    messages: HashMap<String, String>,
}

impl AnalyzerManager {
    fn load_analyzers(&self) {
        &self.analyzers = HashMap::from([
            
        ]);
    }

    fn ensure_analyzer_is_up_async(&self, language)
}