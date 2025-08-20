# Wikipedia Core

A Rust library for generating StaticMCP files from Wikipedia XML dumps. This core library provides the building blocks for creating topic-focused Wikipedia StaticMCP deployments with advanced features like streaming processing and automatic categorization.

## Features

- **🌊 Streaming Parser**: Process massive Wikipedia dumps (22GB+) without local storage
- **🎯 Topic Filtering**: Advanced filtering by History, Science, Technology, Mathematics
- **🏷️ Configurable Categorization**: Real-time article categorization with custom categorizers
- **📑 Pagination**: Efficient browsing with page-based article navigation
- **🔍 Search Optimization**: Pre-generated search results for common queries
- **🌐 UTF-8 Support**: Safe filename encoding for all languages with collision handling
- **📦 StaticMCP Compliance**: Full compatibility with StaticMCP specification
- **⚡ Memory Efficient**: Streaming mode uses ~100MB RAM regardless of dump size

## Usage

### Basic Example

```rust
use wikipedia_core::{Config, TopicFilter, generate, NoCategorizer};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::new(
        PathBuf::from("enwiki-latest-pages-articles.xml.bz2"),
        PathBuf::from("./output")
    )
    .language("en")
    .topic_filter(TopicFilter::History)
    .exact_matches(true)
    .max_articles(10000);

    generate(config, NoCategorizer)?;
    Ok(())
}
```

### Streaming from URL

```rust
use wikipedia_core::{WikipediaParser, StaticMcpGenerator, TopicFilter};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://dumps.wikimedia.org/simplewiki/latest/simplewiki-latest-pages-articles.xml.bz2";
    let output_dir = PathBuf::from("./output");
    
    // Stream processing without downloading the full file
    let response = reqwest::get(url).await?;
    let stream = response.bytes_stream();
    
    // Process stream directly to StaticMCP
    // Implementation details in main.rs
    Ok(())
}
```

### Configuration Options

- **`language()`** - Set Wikipedia language code (default: "en")
- **`topic_filter()`** - Filter by topic: History, Science, Technology, Mathematics
- **`exact_matches()`** - Generate exact match files for all articles (increases size)
- **`max_articles()`** - Limit number of articles processed (useful for testing)

### Topic Filters

```rust
use wikipedia_core::TopicFilter;

// Available filters
let history = TopicFilter::History;
let science = TopicFilter::Science;
let technology = TopicFilter::Technology;
let mathematics = TopicFilter::Mathematics;

// Check if content matches filter
if history.is_relevant("World War II", "The Second World War...") {
    println!("Article matches history filter");
}

// Get filter keywords
let keywords = history.keywords();
println!("History keywords: {:?}", keywords);
```

## Categorization System

The library supports configurable article categorization through the `ArticleCategorizer` trait:

### Built-in Categorizers
- **NoCategorizer**: No categorization (default)
- **Custom Categorizers**: Implement the `ArticleCategorizer` trait

### Custom Categorizer Example

```rust
use wikipedia_core::ArticleCategorizer;

struct CustomCategorizer;

impl ArticleCategorizer for CustomCategorizer {
    fn categorize(&self, title: &str, content: &str) -> Vec<String> {
        let mut categories = Vec::new();
        
        if title.to_lowercase().contains("science") {
            categories.push("science".to_string());
        }
        
        if content.contains("technology") {
            categories.push("technology".to_string());
        }
        
        categories
    }
}

// Use with generator
let generator = StaticMcpGenerator::new(output_dir, "en".to_string(), parser, CustomCategorizer);
```

## Available Tools in Generated StaticMCP

1. **`get_article`** - Retrieve complete article content
2. **`list_articles`** - Paginated article browsing
3. **`list_categories`** - Get available categories
4. **`categories`** - Get articles from specific category

## Advanced Features

### Collision Handling

```rust
// Automatic UTF-8 filename encoding with collision detection
let safe_filename = filename_encoding::encode_staticmcp_filename("François Mitterrand");
// Result: "francois_mitterrand"

// Long names get hash suffixes
let long_filename = filename_encoding::encode_staticmcp_filename(&"A".repeat(300));
// Result: "aaa...aaa_1234567890abcdef" (truncated with hash)
```

### Wikitext Cleaning

```rust
use wikipedia_core::parser::clean_wikitext;

let raw = "'''Bold text''' with [[links]] and {{templates}}.";
let clean = clean_wikitext(raw);
// Result: "Bold text with links and ."
```

## Testing

Run the comprehensive test suite:

```bash
# Run all unit tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test parser::tests

# Test with actual Wikipedia data
cargo test test_generate_staticmcp
```

## Integration Examples

```rust
// In your Cargo.toml
[dependencies]
wikipedia_core = { path = "../wikipedia_core" }

// In your code
use wikipedia_core::{Config, TopicFilter, generate, NoCategorizer};

let config = Config::new(input_path, output_path)
    .topic_filter(TopicFilter::History)
    .max_articles(1000);
    
generate(config, NoCategorizer)?;
```

