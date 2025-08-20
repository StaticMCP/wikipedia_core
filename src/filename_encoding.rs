use unicode_normalization::UnicodeNormalization;

const MAX_FILENAME_LENGTH: usize = 200;

pub fn encode_staticmcp_filename(name: &str) -> String {
    let normalized = normalize_unicode(name);
    let safe_chars = make_filename_safe(&normalized);

    if safe_chars.len() <= MAX_FILENAME_LENGTH {
        safe_chars
    } else {
        create_short_filename(name, &safe_chars)
    }
}

fn normalize_unicode(text: &str) -> String {
    text.nfd().filter(|c| !is_combining_mark(*c)).collect()
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}' | '\u{1AB0}'..='\u{1AFF}' | '\u{1DC0}'..='\u{1DFF}' | '\u{20D0}'..='\u{20FF}' | '\u{FE20}'..='\u{FE2F}')
}

fn make_filename_safe(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' | '-' | '_' => c,
            ' ' => '_',
            _ => '_',
        })
        .collect()
}

fn create_short_filename(original: &str, encoded: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    original.hash(&mut hasher);
    let hash = hasher.finish();

    let prefix_len = MAX_FILENAME_LENGTH - 17; // Leave room for _hash
    format!(
        "{}_{:016x}",
        &encoded[..prefix_len.min(encoded.len())],
        hash
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_encoding() {
        assert_eq!(encode_staticmcp_filename("Hello World"), "hello_world");
        assert_eq!(encode_staticmcp_filename("Test-123"), "test-123");
    }

    #[test]
    fn test_unicode_normalization() {
        assert_eq!(
            encode_staticmcp_filename("François Mitterrand"),
            "francois_mitterrand"
        );
        assert_eq!(encode_staticmcp_filename("José María"), "jose_maria");
        assert_eq!(encode_staticmcp_filename("Björk"), "bjork");
    }

    #[test]
    fn test_long_filename() {
        let long_name = "A".repeat(250);
        let encoded = encode_staticmcp_filename(&long_name);
        assert!(encoded.len() <= MAX_FILENAME_LENGTH);
        assert!(encoded.contains("_"));
    }
}
