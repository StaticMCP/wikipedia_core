use crate::filters::TopicFilter;
use crate::types::Article;
use bzip2::read::BzDecoder;
use quick_xml::Reader;
use quick_xml::events::Event;
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub struct WikipediaParser {
    pub language: String,
    pub articles: HashMap<String, Article>,
    pub redirects: HashMap<String, String>,
}

impl WikipediaParser {
    pub fn new(language: String) -> Self {
        Self {
            language,
            articles: HashMap::new(),
            redirects: HashMap::new(),
        }
    }

    pub fn parse(
        &mut self,
        file_path: &Path,
        max_articles: Option<usize>,
        topic_filter: &Option<TopicFilter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;

        let reader_box: Box<dyn Read> = if file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .as_deref()
            == Some("bz2")
        {
            Box::new(BzDecoder::new(file))
        } else {
            Box::new(file)
        };

        let buf_reader = BufReader::new(reader_box);
        let mut reader = Reader::from_reader(buf_reader);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut current_article = None::<Article>;
        let mut current_content = String::new();
        let mut articles_processed = 0;
        let mut skip_content = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    current_content.clear();

                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "page" {
                        current_article = Some(Article {
                            title: String::new(),
                            content: String::new(),
                            id: 0,
                            redirect: None,
                        });
                        skip_content = false;
                    }
                }
                Ok(Event::Text(e)) => {
                    current_content.push_str(&e.unescape()?);
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if let Some(ref mut article) = current_article {
                        match tag_name.as_ref() {
                            "title" => {
                                article.title = current_content.clone();
                                if !should_include_by_title(&article.title, topic_filter) {
                                    skip_content = true;
                                }
                            }
                            "id" => {
                                if article.id == 0 {
                                    article.id = current_content.parse().unwrap_or(0);
                                }
                            }
                            "text" => {
                                if !skip_content {
                                    article.content = clean_wikitext(&current_content);
                                }
                            }
                            "redirect" => {
                                article.redirect = Some(current_content.clone());
                            }
                            "page" => {
                                if let Some(article) = current_article.take()
                                    && !skip_content
                                    && should_include_by_content(&article, topic_filter)
                                {
                                    if let Some(redirect) = &article.redirect {
                                        self.redirects
                                            .insert(article.title.clone(), redirect.clone());
                                    } else {
                                        self.articles.insert(article.title.clone(), article);
                                    }

                                    articles_processed += 1;
                                    if articles_processed % 1000 == 0 {
                                        println!("Processed {articles_processed} articles...");
                                    }

                                    if let Some(max) = max_articles
                                        && articles_processed >= max
                                    {
                                        break;
                                    }
                                }
                                skip_content = false;
                            }
                            _ => {}
                        }
                    }
                    current_content.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(Box::new(e)),
                _ => {}
            }
            buf.clear();
        }

        println!(
            "Parsed {} articles and {} redirects",
            self.articles.len(),
            self.redirects.len()
        );
        Ok(())
    }

    pub fn parse_streaming<F>(
        &self,
        reader: Box<dyn Read>,
        is_bz2: bool,
        topic_filter: &Option<TopicFilter>,
        mut article_handler: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(&str, &Article) -> Result<(), Box<dyn std::error::Error>>,
    {
        let reader_box: Box<dyn Read> = if is_bz2 {
            Box::new(BzDecoder::new(reader))
        } else {
            reader
        };

        let buf_reader = BufReader::new(reader_box);
        let mut reader = Reader::from_reader(buf_reader);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut current_article = None::<Article>;
        let mut current_content = String::new();
        let mut articles_processed = 0;
        let mut skip_content = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    current_content.clear();

                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if tag_name == "page" {
                        current_article = Some(Article {
                            title: String::new(),
                            content: String::new(),
                            id: 0,
                            redirect: None,
                        });
                        skip_content = false;
                    }
                }
                Ok(Event::Text(e)) => {
                    current_content.push_str(&e.unescape()?);
                }
                Ok(Event::End(ref e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                    if let Some(ref mut article) = current_article {
                        match tag_name.as_ref() {
                            "title" => {
                                article.title = current_content.clone();
                                if !should_include_by_title(&article.title, topic_filter) {
                                    skip_content = true;
                                }
                            }
                            "id" => {
                                if article.id == 0 {
                                    article.id = current_content.parse().unwrap_or(0);
                                }
                            }
                            "text" => {
                                if !skip_content {
                                    article.content = clean_wikitext(&current_content);
                                }
                            }
                            "redirect" => {
                                article.redirect = Some(current_content.clone());
                            }
                            "page" => {
                                if let Some(article) = current_article.take()
                                    && !skip_content
                                    && should_include_by_content(&article, topic_filter)
                                {
                                    article_handler(&article.title, &article)?;
                                    articles_processed += 1;

                                    if articles_processed % 1000 == 0 {
                                        println!("Processed {articles_processed} articles...");
                                    }
                                }
                                skip_content = false;
                            }
                            _ => {}
                        }
                    }
                    current_content.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(Box::new(e)),
                _ => {}
            }
            buf.clear();
        }

        println!("Streaming processing complete: {articles_processed} articles processed");
        Ok(())
    }
}

fn should_include_by_title(title: &str, topic_filter: &Option<TopicFilter>) -> bool {
    if title.is_empty() {
        return false;
    }

    let excluded_prefixes = [
        "File:",
        "Category:",
        "Template:",
        "User:",
        "Talk:",
        "Wikipedia:",
        "Help:",
        "Portal:",
        "MediaWiki:",
        "Module:",
    ];

    if excluded_prefixes
        .iter()
        .any(|&prefix| title.starts_with(prefix))
    {
        return false;
    }

    if let Some(filter) = topic_filter {
        let title_lower = title.to_lowercase();
        filter
            .keywords()
            .iter()
            .any(|&keyword| title_lower.contains(keyword))
    } else {
        true
    }
}

fn should_include_by_content(article: &Article, topic_filter: &Option<TopicFilter>) -> bool {
    if let Some(filter) = topic_filter {
        filter.is_relevant(&article.title, &article.content)
    } else {
        true
    }
}

pub fn clean_wikitext(content: &str) -> String {
    let patterns = [
        (r"\{\{[^}]*\}\}", ""),
        (r"\[\[Category:[^\]]*\]\]", ""),
        (r"\[\[File:[^\]]*\]\]", ""),
        (r"\[\[[^\]]*\|([^\]]*)\]\]", "$1"),
        (r"\[\[([^\]]*)\]\]", "$1"),
        (r"'''([^']*?)'''", "$1"),
        (r"''([^']*?)''", "$1"),
        (r"<ref[^>]*>[^<]*</ref>", ""),
        (r"<nowiki>[^<]*</nowiki>", ""),
        (r"<[^>]*>", ""),
        (r"={2,6}([^=]*?)={2,6}", "$1"),
    ];

    let mut cleaned = content.to_string();
    for (pattern, replacement) in patterns {
        if let Ok(re) = Regex::new(pattern) {
            cleaned = re.replace_all(&cleaned, replacement).to_string();
        }
    }

    cleaned
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
