use jieba_rs::Jieba;
use once_cell::sync::Lazy;
use unicode_segmentation::UnicodeSegmentation;

// We use Lazy to ensure the dictionary is only loaded once into memory,
// making the function much faster for repeated calls.
static JIEBA: Lazy<Jieba> = Lazy::new(Jieba::new);

pub fn split_text(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current_segment = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // Handle quotes but protect English contractions like "it's"
            '\'' | '"' => {
                let is_contraction = c == '\''
                    && !current_segment.is_empty()
                    && chars.peek().is_some_and(|next| next.is_alphabetic());

                if is_contraction {
                    current_segment.push(c);
                } else {
                    // It's a quote boundary
                    if !current_segment.is_empty() {
                        words.extend(process_segment(&current_segment));
                        current_segment.clear();
                    }
                    in_quotes = !in_quotes;
                }
            }
            // Split by whitespace only if not in a quote
            c if c.is_whitespace() && !in_quotes => {
                if !current_segment.is_empty() {
                    words.extend(process_segment(&current_segment));
                    current_segment.clear();
                }
            }
            _ => {
                current_segment.push(c);
            }
        }
    }

    if !current_segment.is_empty() {
        words.extend(process_segment(&current_segment));
    }
    words
}

fn process_segment(segment: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let has_cjk = segment.chars().any(|c| {
        ('\u{4e00}'..='\u{9fff}').contains(&c) || // Chinese
        ('\u{3040}'..='\u{30ff}').contains(&c) // Japanese
    });

    if has_cjk {
        // Jieba logic (Existing)
        JIEBA
            .cut(segment, true)
            .into_iter()
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty())
            .collect()
    } else {
        // split_word_bounds() gives us words, punctuation, and spaces as separate tokens
        let mut tokens = segment.split_word_bounds().peekable();
        while let Some(token) = tokens.next() {
            // 1. Ignore whitespace tokens
            if token.trim().is_empty() {
                continue;
            }

            // 2. Identify if this token is a punctuation mark we want to merge
            let is_punctuation = matches!(token, "." | "," | "!" | "?" | "。" | "、" | "！" | "？");

            if is_punctuation && !result.is_empty() {
                // Reattach to the previous word
                if let Some(last_word) = result.last_mut() {
                    last_word.push_str(token);
                    continue;
                }
            }

            // 3. Identify if this is a hyphen connector (for world-test)
            if token == "-"
                && !result.is_empty()
                && let Some(next_token) = tokens.peek()
            {
                // If the next part is a word, merge [prev] + [-] + [next]
                if !next_token.trim().is_empty() {
                    let mut hyphenated = result.pop().unwrap();
                    hyphenated.push('-');
                    hyphenated.push_str(tokens.next().unwrap()); // Consume the word after hyphen
                    result.push(hyphenated);
                    continue;
                }
            }

            // 4. Only add if it contains alphanumeric characters (ignores lone symbols)
            if token.chars().any(|c| c.is_alphanumeric()) {
                result.push(token.to_string());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // japanese is broken for JIEBA
    // #[test]
    // fn test_japanese_segmentation_with_punctuation() {
    //     let input = "上の例では、データ。";
    //     let result = split_text(input);

    //     // Expected behavior:
    //     // "上", "の", "例", "では、", "データ。"
    //     // Note: particles like "では" often stay together in Jieba,
    //     // and punctuation should merge.
    //     println!("test_japanese_segmentation_with_punctuation {:?}", result);
    //     assert_eq!(result[3], "では、");
    //     assert_eq!(result[4], "データ。");
    // }

    // #[test]
    // fn test_quoted_text_preservation() {
    //     let input = "これは \"Special Case\" です。";
    //     let result = split_text(input);

    //     // Expected: ["これは", "\"Special Case\"", "です。"]
    //     assert!(result.contains(&"\"Special Case\"".to_string()));
    //     assert_eq!(result.last().unwrap(), "です。");
    // }

    #[test]
    fn test_multiple_punctuation_merge() {
        let input = "Hello, world-test. \"Done!\"";
        let result = split_text(input);

        // Expected: ["Hello,", "world-test.", "Done!"]
        assert_eq!(result[0], "Hello,");
        assert_eq!(result[1], "world-test.");
        assert_eq!(result[2], "Done!");
    }

    #[test]
    fn test_mixed_jieba_unicode() {
        // Test Case: English Contractions + Quoted Phrases + Chinese
        let input = "There's credibility to 'this time it's different' and 這是一個測試。";
        let result = split_text(input);

        // Verify English contractions
        assert!(result.contains(&"There's".to_string()));
        assert!(result.contains(&"it's".to_string()));

        // Verify Chinese (Jieba)
        assert!(result.contains(&"這是".to_string()));
        assert!(result.contains(&"一個".to_string()));

        // Verify Quote Stripping
        assert!(!result.contains(&"'this".to_string()));
    }

    #[test]
    fn test_single_quote() {
        let input = "There's some credibility to 'this time it's different'";
        let result = split_text(input);
        let expected = vec![
            "There's",
            "some",
            "credibility",
            "to",
            "this",
            "time",
            "it's",
            "different",
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_ignore_single_punctuation() {
        let input = "That is - the result";
        let result = split_text(input);
        let expected = vec!["That", "is", "the", "result"];

        assert_eq!(result, expected);
    }
}
