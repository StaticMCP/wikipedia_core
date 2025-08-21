use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use wikipedia_core::{ArticleCategorizer, Config, NoCategorizer, TopicFilter, generate};

struct TestCategorizer;

impl ArticleCategorizer for TestCategorizer {
    fn categorize(&self, title: &str, _content: &str) -> Vec<String> {
        let title_lower = title.to_lowercase();
        let mut categories = Vec::new();

        if title_lower.contains("war") {
            categories.push("war".to_string());
        }

        categories
    }
}

fn create_test_xml() -> String {
    r#"<mediawiki>
  <page>
    <title>World War II</title>
    <id>32927</id>
    <revision>
      <text>World War II was a global war that lasted from 1939 to 1945. The war involved the vast majority of the world's countries—including all of the great powers—forming two opposing military alliances: the Allies and the Axis.</text>
    </revision>
  </page>
  <page>
    <title>Roman Empire</title>
    <id>25458</id>
    <revision>
      <text>The Roman Empire was the post-Republican period of ancient Rome. As a polity it included large territorial holdings around the Mediterranean Sea in Europe, Northern Africa, and Western Asia ruled by emperors.</text>
    </revision>
  </page>
  <page>
    <title>File:Example.jpg</title>
    <id>12345</id>
    <revision>
      <text>This is a file page and should be excluded.</text>
    </revision>
  </page>
  <page>
    <title>Computer Science</title>
    <id>5323</id>
    <revision>
      <text>Computer science is the study of algorithms and data structures, computational systems, and the design of computer systems and their applications.</text>
    </revision>
  </page>
</mediawiki>"#.to_string()
}

#[test]
fn test_topic_filter_history() {
    let filter = TopicFilter::History;

    assert!(filter.is_relevant("World War II", "global war"));
    assert!(filter.is_relevant("Roman Empire", "ancient Rome"));
    assert!(!filter.is_relevant("Computer Science", "algorithms"));

    assert_eq!(
        filter.description(),
        "Historical Events, Figures, and Civilizations"
    );
    assert_eq!(filter.server_name("en"), "Wikipedia EN History StaticMCP");
}

#[test]
fn test_topic_filter_technology() {
    let filter = TopicFilter::Technology;

    assert!(filter.is_relevant("Computer Science", "algorithms"));
    assert!(filter.is_relevant("Python Programming", "software development"));
    assert!(!filter.is_relevant("World War II", "global war"));
}

#[test]
fn test_config_builder() {
    let config = Config::new(PathBuf::from("input.xml"), PathBuf::from("output"))
        .language("es")
        .topic_filter(TopicFilter::History)
        .exact_matches(true)
        .max_articles(1000);

    assert_eq!(config.language, "es");
    assert_eq!(config.topic_filter, Some(TopicFilter::History));
    assert!(config.exact_matches);
    assert_eq!(config.max_articles, Some(1000));
}

#[test]
fn test_generate_staticmcp() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("test.xml");
    let output_dir = temp_dir.path().join("output");

    fs::write(&input_file, create_test_xml())?;

    let config = Config::new(input_file, output_dir.clone())
        .language("en")
        .topic_filter(TopicFilter::History)
        .max_articles(10);

    generate(config, NoCategorizer)?;

    assert!(output_dir.join("mcp.json").exists());
    assert!(output_dir.join("resources").exists());
    assert!(output_dir.join("resources/stats.json").exists());
    assert!(output_dir.join("resources/articles.json").exists());
    assert!(output_dir.join("tools").exists());
    assert!(output_dir.join("tools/list_articles").exists());
    assert!(output_dir.join("tools/categories").exists());
    assert!(output_dir.join("tools/get_article").exists());

    let manifest_content = fs::read_to_string(output_dir.join("mcp.json"))?;
    assert!(manifest_content.contains("Wikipedia EN History StaticMCP"));
    assert!(manifest_content.contains("list_articles"));
    assert!(manifest_content.contains("categories"));
    assert!(manifest_content.contains("get_article"));
    assert!(manifest_content.contains("list_categories"));

    let stats_content = fs::read_to_string(output_dir.join("resources/stats.json"))?;
    assert!(stats_content.contains("total_articles"));
    assert!(stats_content.contains("generated_at"));
    assert!(stats_content.contains("Historical Events"));

    Ok(())
}

#[test]
fn test_history_filtering() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("test.xml");
    let output_dir = temp_dir.path().join("output");

    fs::write(&input_file, create_test_xml())?;

    let config = Config::new(input_file, output_dir.clone()).topic_filter(TopicFilter::History);

    generate(config, NoCategorizer)?;

    let articles_content = fs::read_to_string(output_dir.join("resources/articles.json"))?;

    assert!(articles_content.contains("World War II"));
    assert!(articles_content.contains("Roman Empire"));
    assert!(!articles_content.contains("Computer Science"));
    assert!(!articles_content.contains("File:Example.jpg"));

    Ok(())
}

#[test]
fn test_pagination_and_categories() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("test.xml");
    let output_dir = temp_dir.path().join("output");

    fs::write(&input_file, create_test_xml())?;

    let config = Config::new(input_file, output_dir.clone()).topic_filter(TopicFilter::History);

    generate(config, TestCategorizer)?;

    assert!(output_dir.join("tools/list_articles.json").exists());
    assert!(output_dir.join("tools/list_categories.json").exists());

    let list_content = fs::read_to_string(output_dir.join("tools/list_articles.json"))?;
    assert!(list_content.contains("pagination"));
    assert!(list_content.contains("current_page"));
    assert!(list_content.contains("total_articles"));

    let categories_content = fs::read_to_string(output_dir.join("tools/list_categories.json"))?;
    assert!(categories_content.contains("categories"));
    assert!(categories_content.contains("war"));

    let war_category = output_dir.join("tools/categories/war.json");
    if war_category.exists() {
        let war_content = fs::read_to_string(war_category)?;
        assert!(war_content.contains("category"));
        assert!(war_content.contains("articles"));
    }

    Ok(())
}

#[test]
fn test_article_response_generation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("test.xml");
    let output_dir = temp_dir.path().join("output");

    fs::write(&input_file, create_test_xml())?;

    let config = Config::new(input_file, output_dir.clone()).topic_filter(TopicFilter::History);

    generate(config, NoCategorizer)?;

    let article_dir = output_dir.join("tools/get_article");

    assert!(article_dir.join("world_war_ii.json").exists());
    assert!(article_dir.join("roman_empire.json").exists());

    let article_content = fs::read_to_string(article_dir.join("world_war_ii.json"))?;
    assert!(article_content.contains("# World War II"));
    assert!(article_content.contains("global war"));

    Ok(())
}

#[test]
fn test_keyword_matching() {
    let history = TopicFilter::History;
    let technology = TopicFilter::Technology;
    let science = TopicFilter::Science;
    let mathematics = TopicFilter::Mathematics;

    assert!(history.keywords().contains(&"war"));
    assert!(history.keywords().contains(&"empire"));

    assert!(technology.keywords().contains(&"computer"));
    assert!(technology.keywords().contains(&"programming"));

    assert!(science.keywords().contains(&"physics"));
    assert!(science.keywords().contains(&"biology"));

    assert!(mathematics.keywords().contains(&"theorem"));
    assert!(mathematics.keywords().contains(&"algebra"));
}

#[test]
fn test_wikitext_cleaning() {
    use wikipedia_core::parser::clean_wikitext;

    let input = "'''Bold text''' and ''italic text'' with [[links]] and {{templates}}.";
    let cleaned = clean_wikitext(input);
    assert_eq!(cleaned, "Bold text and italic text with links and .");

    let input_with_refs = "Text with <ref>reference</ref> and <nowiki>nowiki</nowiki>.";
    let cleaned = clean_wikitext(input_with_refs);
    assert_eq!(cleaned, "Text with  and .");
}

#[test]
fn test_collision_handling_short_articles() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("test.xml");
    let output_dir = temp_dir.path().join("output");

    let test_xml = r#"<mediawiki>
  <page>
    <title>War Article</title>
    <id>1</id>
    <revision>
      <text>Short content about historical war events.</text>
    </revision>
  </page>
  <page>
    <title>War/Article</title>
    <id>2</id>
    <revision>
      <text>Another short article about war history.</text>
    </revision>
  </page>
</mediawiki>"#;

    fs::write(&input_file, test_xml)?;

    let config = Config::new(input_file, output_dir.clone())
        .language("en")
        .topic_filter(TopicFilter::History);

    generate(config, NoCategorizer)?;

    let merged_file = output_dir.join("tools/get_article/war_article.json");
    assert!(merged_file.exists());

    let merged_content = fs::read_to_string(&merged_file)?;
    assert!(merged_content.contains("War Article"));
    assert!(merged_content.contains("War/Article"));
    assert!(merged_content.contains("---"));

    Ok(())
}

#[test]
fn test_collision_handling_long_articles() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("test.xml");
    let output_dir = temp_dir.path().join("output");

    let long_content1 = format!(
        "This is a long historical war article about ancient battles. {}",
        "a".repeat(1400)
    );
    let long_content2 = format!(
        "This is another long war article about medieval conflicts. {}",
        "b".repeat(1400)
    );

    let test_xml = format!(
        r#"<mediawiki>
  <page>
    <title>Battle Article</title>
    <id>1</id>
    <revision>
      <text>{long_content1}</text>
    </revision>
  </page>
  <page>
    <title>Battle/Article</title>
    <id>2</id>
    <revision>
      <text>{long_content2}</text>
    </revision>
  </page>
</mediawiki>"#
    );

    fs::write(&input_file, test_xml)?;

    let config = Config::new(input_file, output_dir.clone())
        .language("en")
        .topic_filter(TopicFilter::History);

    generate(config, NoCategorizer)?;

    let base_file = output_dir.join("tools/get_article/battle_article.json");
    let variant1_file = output_dir.join("tools/get_article/battle_article_1.json");
    let variant2_file = output_dir.join("tools/get_article/battle_article_2.json");

    assert!(base_file.exists());
    assert!(variant1_file.exists());
    assert!(variant2_file.exists());

    let base_content = fs::read_to_string(&base_file)?;
    assert!(base_content.contains("Multiple articles found"));
    assert!(base_content.contains("Battle Article"));
    assert!(base_content.contains("Battle/Article"));

    let variant1_content = fs::read_to_string(&variant1_file)?;
    let variant2_content = fs::read_to_string(&variant2_file)?;

    assert!(!variant1_content.contains("---"));
    assert!(!variant2_content.contains("---"));
    assert_ne!(variant1_content, variant2_content);
    let all_content = format!("{variant1_content}{variant2_content}");
    assert!(all_content.contains("ancient battles"));
    assert!(all_content.contains("medieval conflicts"));

    Ok(())
}

#[test]
fn test_collision_handling_existing_disambiguation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("test.xml");
    let output_dir = temp_dir.path().join("output");

    let long_content1 = format!(
        "This is a historical empire article about ancient kingdoms. {}",
        "a".repeat(1400)
    );
    let long_content2 = format!(
        "This is another empire article about medieval dynasties. {}",
        "b".repeat(1400)
    );
    let long_content3 = format!(
        "This is a third empire article about colonial rule. {}",
        "c".repeat(1400)
    );

    let test_xml = format!(
        r#"<mediawiki>
  <page>
    <title>Empire Article</title>
    <id>1</id>
    <revision>
      <text>{long_content1}</text>
    </revision>
  </page>
  <page>
    <title>Empire/Article</title>
    <id>2</id>
    <revision>
      <text>{long_content2}</text>
    </revision>
  </page>
  <page>
    <title>Empire_Article</title>
    <id>3</id>
    <revision>
      <text>{long_content3}</text>
    </revision>
  </page>
</mediawiki>"#
    );

    fs::write(&input_file, test_xml)?;

    let config = Config::new(input_file, output_dir.clone())
        .language("en")
        .topic_filter(TopicFilter::History);

    generate(config, NoCategorizer)?;

    let base_file = output_dir.join("tools/get_article/empire_article.json");
    let variant1_file = output_dir.join("tools/get_article/empire_article_1.json");
    let variant2_file = output_dir.join("tools/get_article/empire_article_2.json");
    let variant3_file = output_dir.join("tools/get_article/empire_article_3.json");

    assert!(base_file.exists());
    assert!(variant1_file.exists());
    assert!(variant2_file.exists());
    assert!(variant3_file.exists());

    let base_content = fs::read_to_string(&base_file)?;
    assert!(base_content.contains("Multiple articles found"));
    assert!(base_content.contains("Empire Article"));
    assert!(base_content.contains("Empire/Article"));
    assert!(base_content.contains("Empire_Article"));

    let variant1_content = fs::read_to_string(&variant1_file)?;
    let variant2_content = fs::read_to_string(&variant2_file)?;
    let variant3_content = fs::read_to_string(&variant3_file)?;

    assert!(!variant1_content.contains("---"));
    assert!(!variant2_content.contains("---"));
    assert!(!variant3_content.contains("---"));
    assert_ne!(variant1_content, variant2_content);
    assert_ne!(variant2_content, variant3_content);
    assert_ne!(variant1_content, variant3_content);

    let all_content = format!("{variant1_content}{variant2_content}{variant3_content}");
    assert!(all_content.contains("ancient kingdoms"));
    assert!(all_content.contains("medieval dynasties"));
    assert!(all_content.contains("colonial rule"));

    Ok(())
}

#[test]
fn test_collision_handling_mixed_lengths() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let input_file = temp_dir.path().join("test.xml");
    let output_dir = temp_dir.path().join("output");

    let short_content = "Short historical revolution article.";
    let long_content = format!(
        "This is a long revolution article about democratic movements. {}",
        "a".repeat(1400)
    );

    let test_xml = format!(
        r#"<mediawiki>
  <page>
    <title>Revolution Article</title>
    <id>1</id>
    <revision>
      <text>{short_content}</text>
    </revision>
  </page>
  <page>
    <title>Revolution/Article</title>
    <id>2</id>
    <revision>
      <text>{long_content}</text>
    </revision>
  </page>
</mediawiki>"#
    );

    fs::write(&input_file, test_xml)?;

    let config = Config::new(input_file, output_dir.clone())
        .language("en")
        .topic_filter(TopicFilter::History);

    generate(config, NoCategorizer)?;

    let base_file = output_dir.join("tools/get_article/revolution_article.json");
    let variant1_file = output_dir.join("tools/get_article/revolution_article_1.json");
    let variant2_file = output_dir.join("tools/get_article/revolution_article_2.json");

    assert!(base_file.exists());
    assert!(variant1_file.exists());
    assert!(variant2_file.exists());

    let base_content = fs::read_to_string(&base_file)?;
    assert!(base_content.contains("Multiple articles found"));
    assert!(base_content.contains("Revolution Article"));
    assert!(base_content.contains("Revolution/Article"));

    let variant1_content = fs::read_to_string(&variant1_file)?;
    let variant2_content = fs::read_to_string(&variant2_file)?;

    assert!(!variant1_content.contains("---"));
    assert!(!variant2_content.contains("---"));
    assert_ne!(variant1_content, variant2_content);

    Ok(())
}

#[test]
fn test_filename_encoding_collision() -> Result<(), Box<dyn std::error::Error>> {
    use wikipedia_core::filename_encoding::encode_staticmcp_filename;

    let title1 = "Test/Article";
    let title2 = "Test Article";
    let title3 = "Test_Article";

    let encoded1 = encode_staticmcp_filename(title1);
    let encoded2 = encode_staticmcp_filename(title2);
    let encoded3 = encode_staticmcp_filename(title3);

    assert_eq!(encoded1, "test_article");
    assert_eq!(encoded2, "test_article");
    assert_eq!(encoded3, "test_article");

    Ok(())
}
