use crate::filters::TopicFilter;
use crate::parser::WikipediaParser;
use crate::types::*;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

pub struct StaticMcpGenerator<C: ArticleCategorizer> {
    output_dir: PathBuf,
    language: String,
    articles: std::collections::HashMap<String, Article>,
    redirects: std::collections::HashMap<String, String>,
    article_titles: std::collections::HashSet<String>,
    categories: std::collections::HashMap<String, Vec<String>>,
    categorizer: C,
}

impl<C: ArticleCategorizer> StaticMcpGenerator<C> {
    pub fn new(
        output_dir: PathBuf,
        language: String,
        parser: WikipediaParser,
        categorizer: C,
    ) -> Self {
        let mut categories: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (title, article) in &parser.articles {
            let category_names = categorizer.categorize(title, &article.content);
            for category in category_names {
                categories.entry(category).or_default().push(title.clone());
            }
        }

        Self {
            output_dir,
            language,
            article_titles: parser.articles.keys().cloned().collect(),
            articles: parser.articles,
            redirects: parser.redirects,
            categories,
            categorizer,
        }
    }

    pub fn new_streaming(output_dir: PathBuf, language: String, categorizer: C) -> Self {
        Self {
            output_dir,
            language,
            articles: std::collections::HashMap::new(),
            redirects: std::collections::HashMap::new(),
            article_titles: std::collections::HashSet::new(),
            categories: std::collections::HashMap::new(),
            categorizer,
        }
    }

    pub fn generate(
        &mut self,
        exact_matches: bool,
        topic_filter: Option<TopicFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.create_directories()?;
        self.generate_manifest(&topic_filter)?;
        self.generate_resources(&topic_filter)?;
        self.generate_tools(exact_matches, &topic_filter)?;

        println!("Generated StaticMCP files in: {:?}", self.output_dir);
        Ok(())
    }

    fn create_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.output_dir)?;
        fs::create_dir_all(self.output_dir.join("resources"))?;
        fs::create_dir_all(self.output_dir.join("tools/get_article"))?;
        fs::create_dir_all(self.output_dir.join("tools/list_articles"))?;
        fs::create_dir_all(self.output_dir.join("tools/categories"))?;
        Ok(())
    }

    fn create_streaming_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&self.output_dir)?;
        fs::create_dir_all(self.output_dir.join("resources"))?;
        fs::create_dir_all(self.output_dir.join("tools/get_article"))?;
        fs::create_dir_all(self.output_dir.join("tools/list_articles"))?;
        fs::create_dir_all(self.output_dir.join("tools/categories"))?;
        Ok(())
    }

    fn generate_manifest(
        &self,
        topic_filter: &Option<TopicFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let server_name = if let Some(filter) = topic_filter {
            filter.server_name(&self.language)
        } else {
            format!("Wikipedia {} StaticMCP", self.language.to_uppercase())
        };

        let manifest = Manifest {
            protocol_version: "2024-11-05".to_string(),
            server_info: ServerInfo {
                name: server_name,
                version: "1.0.0".to_string(),
            },
            capabilities: Capabilities {
                resources: vec![
                    Resource {
                        uri: "wikipedia://stats".to_string(),
                        name: "Wikipedia Statistics".to_string(),
                        description: "Statistics about the Wikipedia dump".to_string(),
                        mime_type: "application/json".to_string(),
                    },
                    Resource {
                        uri: "wikipedia://articles".to_string(),
                        name: "Article List".to_string(),
                        description: "List of all available Wikipedia articles".to_string(),
                        mime_type: "application/json".to_string(),
                    },
                ],
                tools: vec![
                    Tool {
                        name: "get_article".to_string(),
                        description: "Get the full content of a specific Wikipedia article"
                            .to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "title": {
                                    "type": "string",
                                    "description": "Article title"
                                }
                            },
                            "required": ["title"]
                        }),
                    },
                    Tool {
                        name: "list_articles".to_string(),
                        description: "List available Wikipedia articles with pagination"
                            .to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "page": {
                                    "type": "integer",
                                    "description": "Page number (1-based, default: 1)",
                                    "minimum": 1
                                }
                            },
                            "required": []
                        }),
                    },
                    Tool {
                        name: "list_categories".to_string(),
                        description: "List available article categories".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {},
                            "required": []
                        }),
                    },
                    Tool {
                        name: "categories".to_string(),
                        description: "Get articles from a specific category".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "category": {
                                    "type": "string",
                                    "description": "Category name"
                                }
                            },
                            "required": ["category"]
                        }),
                    },
                ],
            },
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        let mut file = File::create(self.output_dir.join("mcp.json"))?;
        file.write_all(manifest_json.as_bytes())?;
        Ok(())
    }

    fn generate_resources(
        &self,
        topic_filter: &Option<TopicFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stats = serde_json::json!({
            "total_articles": self.articles.len(),
            "total_redirects": self.redirects.len(),
            "language": self.language,
            "topic_filter": topic_filter.as_ref().map(|f| f.description()),
            "generated_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string()
        });

        let stats_response = ResourceResponse {
            uri: "wikipedia://stats".to_string(),
            mime_type: "application/json".to_string(),
            text: serde_json::to_string_pretty(&stats)?,
        };

        let stats_json = serde_json::to_string_pretty(&stats_response)?;
        let mut file = File::create(self.output_dir.join("resources/stats.json"))?;
        file.write_all(stats_json.as_bytes())?;

        let article_titles: Vec<&String> = self.articles.keys().collect();
        let articles_response = ResourceResponse {
            uri: "wikipedia://articles".to_string(),
            mime_type: "application/json".to_string(),
            text: serde_json::to_string(&article_titles)?,
        };

        let articles_json = serde_json::to_string_pretty(&articles_response)?;
        let mut file = File::create(self.output_dir.join("resources/articles.json"))?;
        file.write_all(articles_json.as_bytes())?;

        Ok(())
    }

    fn generate_tools(
        &mut self,
        exact_matches: bool,
        _topic_filter: &Option<TopicFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let article_limit = if exact_matches {
            self.articles.len()
        } else {
            100.min(self.articles.len())
        };

        self.generate_article_responses(article_limit)?;
        self.generate_list_tools()?;
        Ok(())
    }

    fn generate_article_responses(
        &mut self,
        limit: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let articles_to_process: Vec<(String, Article)> = self
            .articles
            .iter()
            .take(limit)
            .map(|(title, article)| (title.clone(), article.clone()))
            .collect();
        println!(
            "Generating {} article responses...",
            articles_to_process.len()
        );

        for (i, (title, article)) in articles_to_process.iter().enumerate() {
            self.write_article_with_collision_handling(title, article)?;

            if (i + 1) % 1000 == 0 {
                println!("Generated {} article responses...", i + 1);
            }
        }
        Ok(())
    }

    pub fn write_article_with_collision_handling(
        &mut self,
        title: &str,
        article: &Article,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.article_titles.insert(title.to_string());
        let category_names = self.categorizer.categorize(title, &article.content);
        for category in category_names {
            self.categories
                .entry(category)
                .or_default()
                .push(title.to_string());
        }
        let filename = crate::filename_encoding::encode_staticmcp_filename(title);
        let file_path = self
            .output_dir
            .join(format!("tools/get_article/{filename}.json"));

        if file_path.exists() {
            let existing_content = std::fs::read_to_string(&file_path)?;
            let existing_response: ToolResponse = serde_json::from_str(&existing_content)?;
            let existing_text = &existing_response.content[0].text;

            let merged_content = self.merge_with_existing_content(existing_text, title, article)?;

            let response = ToolResponse {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: merged_content,
                }],
            };

            let response_json = serde_json::to_string_pretty(&response)?;
            std::fs::write(&file_path, response_json)?;
        } else {
            let content = format!("# {}\n\n{}", title, article.content);
            let response = ToolResponse {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: content,
                }],
            };

            let response_json = serde_json::to_string_pretty(&response)?;
            std::fs::write(&file_path, response_json)?;
        }

        Ok(())
    }

    pub fn generate_metadata_only(
        &self,
        _exact_matches: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("🏛️  Generating metadata files...");

        self.create_streaming_directories()?;

        let topic_filter = Some(crate::filters::TopicFilter::History);
        let server_name = topic_filter
            .as_ref()
            .map(|f| f.server_name(&self.language))
            .unwrap_or_else(|| format!("Wikipedia {} StaticMCP", self.language.to_uppercase()));

        let manifest = crate::types::Manifest {
            protocol_version: "2024-11-05".to_string(),
            server_info: crate::types::ServerInfo {
                name: server_name,
                version: "1.0.0".to_string(),
            },
            capabilities: crate::types::Capabilities {
                resources: vec![crate::types::Resource {
                    uri: "wikipedia://stats".to_string(),
                    name: "Wikipedia Statistics".to_string(),
                    description: "Statistics about the Wikipedia dump".to_string(),
                    mime_type: "application/json".to_string(),
                }],
                tools: vec![
                    crate::types::Tool {
                        name: "get_article".to_string(),
                        description: "Get the full content of a specific Wikipedia article"
                            .to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "title": {
                                    "type": "string",
                                    "description": "Article title"
                                }
                            },
                            "required": ["title"]
                        }),
                    },
                    crate::types::Tool {
                        name: "list_articles".to_string(),
                        description: "List available Wikipedia articles with pagination"
                            .to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "page": {
                                    "type": "integer",
                                    "description": "Page number (1-based, default: 1)",
                                    "minimum": 1
                                }
                            },
                            "required": []
                        }),
                    },
                    crate::types::Tool {
                        name: "list_categories".to_string(),
                        description: "List all available article categories".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {},
                            "required": []
                        }),
                    },
                    crate::types::Tool {
                        name: "categories".to_string(),
                        description: "Get articles from a specific category".to_string(),
                        input_schema: serde_json::json!({
                            "type": "object",
                            "properties": {
                                "category": {
                                    "type": "string",
                                    "description": "Category name"
                                }
                            },
                            "required": ["category"]
                        }),
                    },
                ],
            },
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(self.output_dir.join("mcp.json"), manifest_json)?;

        let stats = serde_json::json!({
            "total_articles": self.article_titles.len(),
            "language": self.language,
            "topic_filter": "History",
            "generated_at": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            "streaming_mode": true
        });

        let stats_response = crate::types::ResourceResponse {
            uri: "wikipedia://stats".to_string(),
            mime_type: "application/json".to_string(),
            text: serde_json::to_string_pretty(&stats)?,
        };

        let stats_json = serde_json::to_string_pretty(&stats_response)?;
        std::fs::write(self.output_dir.join("resources/stats.json"), stats_json)?;

        let article_titles: Vec<&String> = self.article_titles.iter().collect();
        let articles_response = crate::types::ResourceResponse {
            uri: "wikipedia://articles".to_string(),
            mime_type: "application/json".to_string(),
            text: serde_json::to_string(&article_titles)?,
        };

        let articles_json = serde_json::to_string_pretty(&articles_response)?;
        std::fs::write(
            self.output_dir.join("resources/articles.json"),
            articles_json,
        )?;

        self.generate_streaming_pagination()?;
        self.generate_streaming_categories()?;

        println!("✅ Metadata, pagination, and categories generated");
        Ok(())
    }

    fn generate_streaming_pagination(&self) -> Result<(), Box<dyn std::error::Error>> {
        let articles_per_page = 50;
        let all_articles: Vec<&String> = self.article_titles.iter().collect();
        let total_pages = all_articles.len().div_ceil(articles_per_page);

        for page in 1..=total_pages {
            let start_idx = (page - 1) * articles_per_page;
            let end_idx = (start_idx + articles_per_page).min(all_articles.len());
            let page_articles = &all_articles[start_idx..end_idx];

            let page_response = serde_json::json!({
                "pagination": {
                    "current_page": page,
                    "total_pages": total_pages,
                    "per_page": articles_per_page,
                    "total_articles": self.article_titles.len()
                },
                "articles": page_articles
            });

            let response = crate::types::ToolResponse {
                content: vec![crate::types::ToolContent {
                    content_type: "text".to_string(),
                    text: serde_json::to_string_pretty(&page_response)?,
                }],
            };

            let response_json = serde_json::to_string_pretty(&response)?;
            std::fs::write(
                self.output_dir
                    .join(format!("tools/list_articles/{page}.json")),
                response_json,
            )?;
        }

        let metadata_response = serde_json::json!({
            "pagination": {
                "current_page": null,
                "total_pages": total_pages,
                "per_page": articles_per_page,
                "total_articles": self.article_titles.len()
            },
            "message": format!("Use /list_articles/{{page}}.json to get specific pages (1-{})", total_pages)
        });

        let response = crate::types::ToolResponse {
            content: vec![crate::types::ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&metadata_response)?,
            }],
        };

        let response_json = serde_json::to_string_pretty(&response)?;
        std::fs::write(
            self.output_dir.join("tools/list_articles.json"),
            response_json,
        )?;

        Ok(())
    }

    fn generate_streaming_categories(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Generate list_categories.json
        let category_names: Vec<&String> = self.categories.keys().collect();
        let categories_response = serde_json::json!({
            "categories": category_names
        });

        let response = crate::types::ToolResponse {
            content: vec![crate::types::ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&categories_response)?,
            }],
        };

        let response_json = serde_json::to_string_pretty(&response)?;
        std::fs::write(
            self.output_dir.join("tools/list_categories.json"),
            response_json,
        )?;

        // Generate individual category files
        for (category, articles) in &self.categories {
            if !articles.is_empty() {
                let category_response = serde_json::json!({
                    "category": category,
                    "articles": articles,
                    "count": articles.len()
                });

                let response = crate::types::ToolResponse {
                    content: vec![crate::types::ToolContent {
                        content_type: "text".to_string(),
                        text: serde_json::to_string_pretty(&category_response)?,
                    }],
                };

                let response_json = serde_json::to_string_pretty(&response)?;
                std::fs::write(
                    self.output_dir
                        .join(format!("tools/categories/{category}.json")),
                    response_json,
                )?;
            }
        }

        Ok(())
    }

    fn merge_with_existing_content(
        &self,
        existing_text: &str,
        new_title: &str,
        new_article: &Article,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Check if existing content is already a disambiguation page
        if existing_text.starts_with("Multiple articles found") {
            // Add to existing disambiguation
            let new_entry =
                format!("• **{new_title}** - Use get_article tool with title '{new_title}'\n");
            Ok(format!("{existing_text}{new_entry}"))
        } else if existing_text.len() <= 1000 && new_article.content.len() <= 1000 {
            // Both articles are short, create merged content
            Ok(format!(
                "{}\n\n---\n\n## {}\n\n{}",
                existing_text, new_title, new_article.content
            ))
        } else {
            // Convert to reference disambiguation
            let existing_title = self.extract_title_from_content(existing_text);
            Ok(format!(
                "Multiple articles found. Choose the one you need:\n\n• **{existing_title}** - Use get_article tool with title '{existing_title}'\n• **{new_title}** - Use get_article tool with title '{new_title}'\n"
            ))
        }
    }

    fn extract_title_from_content(&self, content: &str) -> String {
        if content.starts_with("# ") {
            content
                .lines()
                .next()
                .unwrap_or("")
                .strip_prefix("# ")
                .unwrap_or("Unknown")
                .to_string()
        } else {
            "Unknown".to_string()
        }
    }

    fn generate_list_tools(&self) -> Result<(), Box<dyn std::error::Error>> {
        let articles_per_page = 50;
        let total_pages = self.articles.len().div_ceil(articles_per_page);
        let all_articles: Vec<&String> = self.articles.keys().collect();
        for page in 1..=total_pages {
            let start_idx = (page - 1) * articles_per_page;
            let end_idx = (start_idx + articles_per_page).min(all_articles.len());
            let page_articles = &all_articles[start_idx..end_idx];

            let page_response = serde_json::json!({
                "pagination": {
                    "current_page": page,
                    "total_pages": total_pages,
                    "per_page": articles_per_page,
                    "total_articles": self.articles.len()
                },
                "articles": page_articles
            });

            let response = ToolResponse {
                content: vec![ToolContent {
                    content_type: "text".to_string(),
                    text: serde_json::to_string_pretty(&page_response)?,
                }],
            };

            let response_json = serde_json::to_string_pretty(&response)?;
            let mut file = File::create(
                self.output_dir
                    .join(format!("tools/list_articles/{page}.json")),
            )?;
            file.write_all(response_json.as_bytes())?;
        }

        let metadata_response = serde_json::json!({
            "pagination": {
                "current_page": null,
                "total_pages": total_pages,
                "per_page": articles_per_page,
                "total_articles": self.articles.len()
            },
            "message": format!("Use /list_articles/{{page}}.json to get specific pages (1-{})", total_pages)
        });

        let response = ToolResponse {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&metadata_response)?,
            }],
        };

        let response_json = serde_json::to_string_pretty(&response)?;
        let mut file = File::create(self.output_dir.join("tools/list_articles.json"))?;
        file.write_all(response_json.as_bytes())?;

        // Generate categories using the same logic as streaming mode
        let category_names: Vec<&String> = self.categories.keys().collect();
        let categories_response = serde_json::json!({
            "categories": category_names
        });

        let response = ToolResponse {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: serde_json::to_string_pretty(&categories_response)?,
            }],
        };

        let response_json = serde_json::to_string_pretty(&response)?;
        let mut file = File::create(self.output_dir.join("tools/list_categories.json"))?;
        file.write_all(response_json.as_bytes())?;

        // Generate individual category files
        for (category, articles) in &self.categories {
            if !articles.is_empty() {
                let category_response = serde_json::json!({
                    "category": category,
                    "articles": articles,
                    "count": articles.len()
                });

                let response = ToolResponse {
                    content: vec![ToolContent {
                        content_type: "text".to_string(),
                        text: serde_json::to_string_pretty(&category_response)?,
                    }],
                };

                let response_json = serde_json::to_string_pretty(&response)?;
                let mut file = File::create(
                    self.output_dir
                        .join(format!("tools/categories/{category}.json")),
                )?;
                file.write_all(response_json.as_bytes())?;
            }
        }

        Ok(())
    }
}
