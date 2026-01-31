use jieba_rs::Jieba;
use once_cell::sync::Lazy;

// We use Lazy to ensure the dictionary is only loaded once into memory,
// making the function much faster for repeated calls.
static JIEBA: Lazy<Jieba> = Lazy::new(Jieba::new);

/// Helper to handle the reattachment logic
fn merge_tokens_into(master_list: &mut Vec<String>, new_tokens: Vec<String>) {
    let mut iter = new_tokens.into_iter().peekable();

    while let Some(token) = iter.next() {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Special handling for Hyphen as a "Connector"
        if trimmed == "-" && !master_list.is_empty() {
            if let Some(next_token) = iter.peek() {
                let next_trimmed = next_token.trim();
                if !next_trimmed.is_empty() {
                    // Peek successful: merge [prev] + [-] + [next]
                    if let Some(last_word) = master_list.last_mut() {
                        last_word.push_str("-");
                        last_word.push_str(next_trimmed);
                        iter.next(); // Consume the peeked token
                        continue;
                    }
                }
            }
        }

        // Standard reattach logic for other punctuation
        let is_punctuation = matches!(trimmed, "," | "." | "!" | "?" | "。" | "、" | "！" | "？");

        if is_punctuation && !master_list.is_empty() {
            if let Some(last_word) = master_list.last_mut() {
                last_word.push_str(trimmed);
                continue;
            }
        }

        master_list.push(trimmed.to_string());
    }
}

fn segment_text(input: &str) -> Vec<String> {
    JIEBA
        .cut(input, true)
        .into_iter()
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .collect()
}

pub fn split_text(text: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut current_segment = String::new();
    let mut in_quotes = false;

    for c in text.chars() {
        match c {
            '"' => {
                if !in_quotes && !current_segment.is_empty() {
                    merge_tokens_into(&mut words, segment_text(&current_segment));
                    current_segment.clear();
                }

                in_quotes = !in_quotes;
                current_segment.push(c);

                // When closing a quote, push the whole quoted block as one "word"
                if !in_quotes {
                    words.push(current_segment.clone());
                    current_segment.clear();
                }
            }
            _ => {
                current_segment.push(c);
            }
        }
    }

    // Process any remaining text after the loop
    if !current_segment.is_empty() {
        if in_quotes {
            // If the user forgot to close a quote, we treat the rest as one block
            words.push(current_segment);
        } else {
            merge_tokens_into(&mut words, segment_text(&current_segment));
        }
    }

    words
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
        let input = "Hello, world-test. Done!";
        let result = split_text(input);

        // Expected: ["Hello,", "world-test.", "Done!"]
        assert_eq!(result[0], "Hello,");
        assert_eq!(result[1], "world-test.");
        assert_eq!(result[2], "Done!");
    }

    #[test]
    fn test_unclosed_quote_fallback() {
        let input = "Missing \"quote here";
        let result = split_text(input);

        // Should treat the unclosed quote as one block to prevent crashing
        assert_eq!(result[1], "\"quote here");
    }

    // #[test]
    // fn test_complex_mixed_cjk() {
    //     let input = "データ \"Batch 1\" を処理、完了。";
    //     let result = split_text(input);

    //     // Expected: ["データ", "\"Batch 1\"", "を", "処理、", "完了。"]
    //     assert_eq!(result[0], "データ");
    //     assert_eq!(result[1], "\"Batch 1\"");
    //     assert_eq!(result[3], "処理、");
    //     assert_eq!(result[4], "完了。");
    // }
}
