use std::path::PathBuf;

pub mod filename_encoding;
pub mod filters;
pub mod generator;
pub mod parser;
pub mod types;

pub use filters::TopicFilter;
pub use generator::StaticMcpGenerator;
pub use parser::WikipediaParser;
pub use types::*;

#[derive(Debug, Clone)]
pub struct Config {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub language: String,
    pub max_articles: Option<usize>,
    pub topic_filter: Option<TopicFilter>,
    pub exact_matches: bool,
}

impl Config {
    pub fn new(input_path: PathBuf, output_path: PathBuf) -> Self {
        Self {
            input_path,
            output_path,
            language: "en".to_string(),
            max_articles: None,
            topic_filter: None,
            exact_matches: false,
        }
    }

    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    pub fn max_articles(mut self, max: usize) -> Self {
        self.max_articles = Some(max);
        self
    }

    pub fn topic_filter(mut self, filter: TopicFilter) -> Self {
        self.topic_filter = Some(filter);
        self
    }

    pub fn exact_matches(mut self, enabled: bool) -> Self {
        self.exact_matches = enabled;
        self
    }
}

pub fn generate<C: ArticleCategorizer>(
    config: Config,
    categorizer: C,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = WikipediaParser::new(config.language.clone());

    let extension = config
        .input_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "xml" | "bz2" => {
            parser.parse(
                &config.input_path,
                config.max_articles,
                &config.topic_filter,
            )?;
        }
        _ => return Err("Unsupported file format. Use .xml or .bz2 files.".into()),
    }

    let mut generator =
        StaticMcpGenerator::new(config.output_path, config.language, parser, categorizer);
    generator.generate(config.exact_matches, config.topic_filter)?;

    Ok(())
}
