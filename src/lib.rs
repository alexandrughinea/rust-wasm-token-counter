use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::File;
use std::cell::RefCell;
use std::collections::HashSet;
use regex::Regex;

pub const DEFAULT_TOKEN_REGEX: &str = r"\b[\p{L}\p{N}]+\b|[^\s\p{L}\p{N}]";

// Regular expressions for token matching
thread_local! {
    static GLOBAL_REGEX: RefCell<Regex> = RefCell::new(Regex::new(DEFAULT_TOKEN_REGEX).unwrap());
}

#[wasm_bindgen]
pub struct TokenCount {
    pub total: u32,
    pub unique: u32,
}

#[cfg(target_arch = "wasm32")]
fn process_with_regex<F, T>(f: F) -> T
where
    F: FnOnce(&Regex) -> T,
{
    GLOBAL_REGEX.with(|re| f(&re.borrow()))
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::DEFAULT_TOKEN_REGEX;
    use lazy_static::lazy_static;
    use regex::Regex;
    use std::sync::RwLock;

    lazy_static! {
        static ref REGEX: RwLock<Regex> = RwLock::new(Regex::new(DEFAULT_TOKEN_REGEX).unwrap());
    }


    pub fn process_with_regex<F, T>(f: F) -> T
    where
        F: FnOnce(&Regex) -> T,
    {
        let regex = REGEX.read().expect("Failed to acquire read lock");
        f(&regex)
    }
}

#[cfg(not(target_arch = "wasm32"))]
use native::process_with_regex;

#[wasm_bindgen]
pub struct TokenProcessor {
    total: u32,
    unique_tokens: HashSet<String>,
}

impl Default for TokenProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl TokenProcessor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> TokenProcessor {
        TokenProcessor {
            total: 0,
            unique_tokens: HashSet::new(),
        }
    }

    pub fn process_chunk(&mut self, chunk: &str) {
        process_with_regex(|regex| {
            for token in regex.find_iter(chunk) {
                let token_text = token.as_str().trim();
                
                if !token_text.is_empty() {
                    self.total += 1;
                    self.unique_tokens.insert(token_text.to_lowercase());
                }
            }
        });
    }

    pub fn get_results(&self) -> TokenCount {
        TokenCount {
            total: self.total,
            unique: self.unique_tokens.len() as u32,
        }
    }
}

#[wasm_bindgen]
pub struct ProcessingProgress {
    pub progress: f64,
    pub total: u32,
    pub unique: u32,
}

#[wasm_bindgen]
pub async fn count_tokens_from_file(file: File, callback: &js_sys::Function) -> Result<TokenCount, JsValue> {
    let mut processor = TokenProcessor::new();
    let file_size = file.size() as usize;
    let mut offset = 0;

    while offset < file_size {
        let end: usize = std::cmp::min(offset + 1024 * 1024, file_size);
        let blob = file.slice_with_i32_and_i32(offset as i32, end as i32)
            .map_err(|_| JsValue::from_str("Failed to slice file"))?;
        
        let array_buffer = JsFuture::from(blob.array_buffer()).await?;
        let uint8_array = Uint8Array::new(&array_buffer);
        let chunk_bytes = uint8_array.to_vec();
        
        let chunk_text = String::from_utf8_lossy(&chunk_bytes).into_owned();

        processor.process_chunk(&chunk_text);

        // Calculate and report progress
        let progress = (offset as f64 / file_size as f64) * 100.0;
        let current_results = processor.get_results();
        let progress_data = ProcessingProgress {
            progress,
            total: current_results.total,
            unique: current_results.unique,
        };
        
        callback.call1(&JsValue::null(), &JsValue::from(progress_data))
            .map_err(|_| JsValue::from_str("Failed to call progress callback"))?;

        offset = end;
    }

    Ok(processor.get_results())
}

#[wasm_bindgen]
pub fn count_tokens_from_raw_text(text: &str, limit: usize) -> TokenCount {
    let mut unique_tokens = HashSet::new();
    let mut total_tokens = 0;

    let mut start = 0;
    while start < text.len() {
        let end = std::cmp::min(start + limit, text.len());
        let chunk = &text[start..end];

        process_with_regex(|regex| {
            for token in regex.find_iter(chunk) {
                let token_text = token.as_str().trim();
                if !token_text.is_empty() {
                    total_tokens += 1;
                    unique_tokens.insert(token_text.to_lowercase());
                }
            }
        });

        start = end;
    }

    TokenCount {
        total: total_tokens,
        unique: unique_tokens.len() as u32,
    }
}

#[wasm_bindgen]
pub async fn get_file_preview(file: File, max_length: usize) -> Result<String, JsValue> {
    let array_buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await?;
    let uint8_array = Uint8Array::new(&array_buffer);
    let bytes = uint8_array.to_vec();

    match std::str::from_utf8(&bytes) {
        Ok(text) => {
            let preview = if text.len() > max_length {
                let mut end = max_length;
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &text[..end])
            } else {
                text.to_string()
            };
            Ok(preview)
        }
        Err(_) => Err(JsValue::from_str("Failed to decode file as UTF-8")),
    }
}

#[wasm_bindgen]
pub fn set_token_regex(pattern: &str) -> Result<(), JsValue> {
    match Regex::new(pattern) {
        Ok(regex) => {
            GLOBAL_REGEX.with(|r| *r.borrow_mut() = regex);
            Ok(())
        }
        Err(e) => Err(JsValue::from_str(&format!("Invalid regex pattern: {}", e))),
    }
}

#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use super::*;

    const RAW_TEXT_LIMIT: usize = 1024;

    #[cfg(not(target_arch = "wasm32"))]
    fn init_test_regex() {
        set_token_regex(DEFAULT_TOKEN_REGEX).unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    fn init_test_regex() {
        set_token_regex(DEFAULT_TOKEN_REGEX).unwrap();
    }

    #[test]
    fn test_count_tokens_basic() {
        init_test_regex();
        let text = "Hello, World! This is a test.";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 9); // Hello , World ! This is a test .
        assert_eq!(result.unique, 9);
    }

    #[test]
    fn test_count_tokens_with_duplicates() {
        init_test_regex();
        let text = "the quick brown fox jumps over the lazy fox";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 9); // 'the' and 'fox' appear twice, so we count them once
        assert_eq!(result.unique, 7); // 'the' and 'fox' are duplicates
    }

    #[test]
    fn test_count_tokens_case_insensitive() {
        init_test_regex();
        let text = "The THE the tHe ThE";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 5);
        assert_eq!(result.unique, 1); // all variations of 'the' count as one unique token
    }

    #[test]
    fn test_count_tokens_special_chars() {
        init_test_regex();
        let text = "Hello! @#$ World... ???";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 12); // Hello ! @ # $ World . . . ? ? ?
        assert_eq!(result.unique, 8); // Hello, !, @, #, $, World, ., ?
    }

    #[test]
    fn test_count_tokens_empty() {
        init_test_regex();
        let text = "";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 0);
        assert_eq!(result.unique, 0);
    }

    #[test]
    fn test_count_tokens_whitespace() {
        init_test_regex();
        let text = "   \n\t   \r\n";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 0);
        assert_eq!(result.unique, 0);
    }

    #[test]
    fn test_token_processor() {
        init_test_regex();
        let mut processor = TokenProcessor::new();
        
        // Process first chunk
        processor.process_chunk("Hello world");
        let result1 = processor.get_results();
        assert_eq!(result1.total, 2);
        assert_eq!(result1.unique, 2);

        // Process second chunk with duplicate
        processor.process_chunk("hello universe");
        let result2 = processor.get_results();
        assert_eq!(result2.total, 4);
        assert_eq!(result2.unique, 3); // 'hello' is counted twice but only once as unique
    }

    #[test]
    fn test_custom_regex_pattern() {
        init_test_regex();
        
        // Test with a pattern that matches individual words
        let pattern = r"\b[A-Za-z]+\b";
        set_token_regex(pattern).unwrap();
        
        let text = "The quick brown fox jumps over the lazy dog";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 9, "Expected 9 tokens (all words)");
        assert_eq!(result.unique, 8, "Expected 8 unique tokens (case-insensitive, 'the' appears twice)");
        
        // Test with a more complex pattern that includes word boundaries and captures punctuation
        let pattern = r"\b[A-Za-z]+\b|[^\s\w]";
        set_token_regex(pattern).unwrap();
        
        let text = "Hello, world! How are you?";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 8, "Expected 8 tokens (Hello , world ! How are you ?)");
        assert_eq!(result.unique, 8, "Expected 8 unique tokens");
        
        // Reset to default pattern
        init_test_regex();
    }

    #[test]
    fn test_invalid_regex_pattern() {
        #[cfg(target_arch = "wasm32")]
        {
            let result = set_token_regex("[invalid");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_unicode_tokens() {
        init_test_regex();
        let text = "Hello 世界! こんにちは";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 4); // Hello, 世界, !, こんにちは
        assert_eq!(result.unique, 4);
    }

    #[test]
    fn test_numbers_as_tokens() {
        init_test_regex();
        let text = "Testing 123 456.789 0.1";
        let result = count_tokens_from_raw_text(text, RAW_TEXT_LIMIT);
        assert_eq!(result.total, 8); // Testing, 123, 456, ., 789, 0, ., 1
        assert_eq!(result.unique, 7); // '.' appears twice but counts as one unique token
    }
}