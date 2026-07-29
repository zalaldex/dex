use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Word,
    Sentence,
    Paragraph,
    Full,
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Word" => Ok(Mode::Word),
            "Sentence" => Ok(Mode::Sentence),
            "Paragraph" => Ok(Mode::Paragraph),
            "Full" => Ok(Mode::Full),
            _ => Err(format!("Unknown mode: {}", s)),
        }
    }
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Word => write!(f, "Word"),
            Mode::Sentence => write!(f, "Sentence"),
            Mode::Paragraph => write!(f, "Paragraph"),
            Mode::Full => write!(f, "Full"),
        }
    }
}

pub struct ModeManager;

impl ModeManager {
    pub fn new() -> Self {
        ModeManager
    }

    pub fn format(&self, text: &str, mode: &str) -> Vec<String> {
        match Mode::from_str(mode) {
            Ok(Mode::Word) => Self::format_by_word(text),
            Ok(Mode::Sentence) => Self::format_by_sentence(text),
            Ok(Mode::Paragraph) => Self::format_by_paragraph(text),
            Ok(Mode::Full) => Self::format_full(text),
            Err(_) => Self::format_by_word(text),
        }
    }

    fn format_by_word(text: &str) -> Vec<String> {
        let words: Vec<&str> = text
            .split_whitespace()
            .collect();

        if words.is_empty() {
            return vec!["```\n```".to_string()];
        }

        let mut result = Vec::new();
        let mut current_message = String::from("```\n");
        const MAX_LEN: usize = 4096 - 10;

        for word in words {
            let word_with_newline = format!("{}\n", word);

            if current_message.len() + word_with_newline.len() + 3 <= MAX_LEN {
                current_message.push_str(&word_with_newline);
            } else {
                current_message.push_str("```");
                result.push(current_message);
                current_message = format!("```\n{}\n", word);
            }
        }

        if current_message != "```\n" {
            current_message.push_str("```");
            result.push(current_message);
        }

        result
    }

    fn format_by_sentence(text: &str) -> Vec<String> {
        let sentences = Self::split_sentences(text);

        if sentences.is_empty() {
            return vec!["```\n```".to_string()];
        }

        let mut result = Vec::new();
        let mut current_message = String::from("```\n");
        const MAX_LEN: usize = 4096 - 10;

        for sentence in sentences {
            let sentence_with_newline = format!("{}\n", sentence.trim());

            if current_message.len() + sentence_with_newline.len() + 3 <= MAX_LEN {
                current_message.push_str(&sentence_with_newline);
            } else {
                current_message.push_str("```");
                result.push(current_message);
                current_message = format!("```\n{}\n", sentence.trim());
            }
        }

        if current_message != "```\n" {
            current_message.push_str("```");
            result.push(current_message);
        }

        result
    }

    fn format_by_paragraph(text: &str) -> Vec<String> {
        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect();

        if paragraphs.is_empty() {
            return vec!["```\n```".to_string()];
        }

        let mut result = Vec::new();
        const MAX_LEN: usize = 4096 - 10;

        for paragraph in paragraphs {
            let formatted = format!("```\n{}\n```", paragraph.trim());

            if formatted.len() <= MAX_LEN {
                result.push(formatted);
            } else {
                let mut current_message = String::from("```\n");
                for line in paragraph.lines() {
                    let line_with_newline = format!("{}\n", line);
                    if current_message.len() + line_with_newline.len() + 3 <= MAX_LEN {
                        current_message.push_str(&line_with_newline);
                    } else {
                        current_message.push_str("```");
                        result.push(current_message);
                        current_message = format!("```\n{}\n", line);
                    }
                }
                if current_message != "```\n" {
                    current_message.push_str("```");
                    result.push(current_message);
                }
            }
        }

        result
    }

    fn format_full(text: &str) -> Vec<String> {
        let trimmed = text.trim();
        const MAX_LEN: usize = 4096 - 10;

        if trimmed.len() + 6 <= MAX_LEN {
            return vec![format!("```\n{}\n```", trimmed)];
        }

        let mut result = Vec::new();
        let mut current_chunk = String::new();

        for ch in trimmed.chars() {
            if current_chunk.len() + 1 + 6 <= MAX_LEN {
                current_chunk.push(ch);
            } else {
                result.push(format!("```\n{}\n```", current_chunk));
                current_chunk = ch.to_string();
            }
        }

        if !current_chunk.is_empty() {
            result.push(format!("```\n{}\n```", current_chunk));
        }

        result
    }

    fn split_sentences(text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();

        for ch in text.chars() {
            current.push(ch);
            if ch == '.' || ch == '!' || ch == '?' {
                if !current.trim().is_empty() {
                    sentences.push(current.trim().to_string());
                }
                current.clear();
            }
        }

        if !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
        }

        sentences
    }
}

impl Default for ModeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_by_word() {
        let text = "Hello world test";
        let result = ModeManager::format_by_word(text);
        assert!(!result.is_empty());
        assert!(result[0].contains("Hello"));
    }

    #[test]
    fn test_format_by_sentence() {
        let text = "Hello world. This is a test.";
        let result = ModeManager::format_by_sentence(text);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_format_by_paragraph() {
        let text = "First paragraph.\n\nSecond paragraph.";
        let result = ModeManager::format_by_paragraph(text);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_format_full() {
        let text = "Hello world";
        let result = ModeManager::format_full(text);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains("Hello world"));
    }
}
