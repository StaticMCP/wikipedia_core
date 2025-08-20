use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub content: String,
    pub id: u64,
    pub redirect: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    pub capabilities: Capabilities,
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct Capabilities {
    pub resources: Vec<Resource>,
    pub tools: Vec<Tool>,
}

#[derive(Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

#[derive(Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

#[derive(Serialize, Deserialize)]
pub struct ResourceResponse {
    pub uri: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
pub struct ToolResponse {
    pub content: Vec<ToolContent>,
}

#[derive(Serialize, Deserialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// Trait for customizable article categorization
pub trait ArticleCategorizer {
    /// Categorize an article based on its title and content
    /// Returns a vector of category names that this article belongs to
    fn categorize(&self, title: &str, content: &str) -> Vec<String>;
}

/// Default no-op categorizer that doesn't categorize articles
pub struct NoCategorizer;

impl ArticleCategorizer for NoCategorizer {
    fn categorize(&self, _title: &str, _content: &str) -> Vec<String> {
        Vec::new()
    }
}
