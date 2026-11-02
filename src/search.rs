//! Full text search over the posts. The blog is small, so every search simply
//! scans the text of every post, which takes well under a millisecond.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// Longest query used; the rest is ignored.
const MAX_QUERY_CHARS: usize = 100;
/// Most words used from a query.
const MAX_QUERY_WORDS: usize = 10;
/// Characters of context shown around a match, before and after it.
const CONTEXT_BEFORE: usize = 60;
const CONTEXT_AFTER: usize = 120;

/// Lowercase and without accents, so that "man quan" finds "Màn quán".
#[must_use]
pub fn normalize(text: &str) -> String {
    text.nfd()
        .filter(|&c| !is_combining_mark(c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Byte index of the first place in normalized `text` where normalized `word`
/// starts a word, so "man" finds "many" but not "woman". Characters other
/// than ASCII letters and digits, such as Chinese ones, match anywhere.
#[must_use]
pub fn find_word(text: &str, word: &str) -> Option<usize> {
    let starts_word = |index: usize| {
        let first = word.chars().next();
        let previous = text[..index].chars().next_back();
        !(first.is_some_and(|c| c.is_ascii_alphanumeric())
            && previous.is_some_and(|c| c.is_ascii_alphanumeric()))
    };
    text.match_indices(word)
        .map(|(index, _)| index)
        .find(|&index| starts_word(index))
}

/// The words of a search query, normalized, without duplicates.
#[must_use]
pub fn query_words(query: &str) -> Vec<String> {
    let query: String = query.chars().take(MAX_QUERY_CHARS).collect();
    let mut words: Vec<String> = Vec::new();
    for word in normalize(&query).split_whitespace() {
        if !words.iter().any(|known| known == word) {
            words.push(word.to_string());
        }
    }
    words.truncate(MAX_QUERY_WORDS);
    words
}

/// Text prepared for searching: matches are found in the normalized text and
/// mapped back to the original, so excerpts keep their accents.
#[derive(Clone)]
pub struct SearchText {
    original: Vec<char>,
    normalized: String,
    /// For each character of `normalized`, the index of the original
    /// character it came from.
    origin: Vec<usize>,
}

/// An excerpt of a text around a match, to show the match highlighted.
#[derive(Debug, PartialEq, Eq)]
pub struct Excerpt {
    pub before: String,
    pub matched: String,
    pub after: String,
}

impl SearchText {
    #[must_use]
    pub fn new(text: &str) -> Self {
        let original: Vec<char> = text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .collect();
        let mut normalized = String::new();
        let mut origin = Vec::new();
        for (index, &c) in original.iter().enumerate() {
            for n in normalize(&c.to_string()).chars() {
                normalized.push(n);
                origin.push(index);
            }
        }
        SearchText {
            original,
            normalized,
            origin,
        }
    }

    /// Whether a word in the text starts with the normalized `word`, see [`find_word`].
    #[must_use]
    pub fn contains(&self, word: &str) -> bool {
        find_word(&self.normalized, word).is_some()
    }

    /// The text around the first place where the normalized `word` appears,
    /// cut at word boundaries.
    #[must_use]
    pub fn excerpt(&self, word: &str) -> Option<Excerpt> {
        let byte_start = find_word(&self.normalized, word)?;
        let char_start = self.normalized[..byte_start].chars().count();
        let char_end = char_start + word.chars().count();
        let start = self.origin[char_start];
        let end = self.origin[char_end - 1] + 1;

        let mut from = start.saturating_sub(CONTEXT_BEFORE);
        if from > 0 {
            // Start at the next word instead of in the middle of one.
            from = (from..start)
                .find(|&i| self.original[i] == ' ')
                .map_or(start, |i| i + 1);
        }
        let mut to = (end + CONTEXT_AFTER).min(self.original.len());
        if to < self.original.len() {
            to = (end..to)
                .rev()
                .find(|&i| self.original[i] == ' ')
                .unwrap_or(end);
        }

        let text = |range: std::ops::Range<usize>| self.original[range].iter().collect::<String>();
        Some(Excerpt {
            before: format!("{}{}", if from > 0 { "…" } else { "" }, text(from..start)),
            matched: text(start..end),
            after: format!(
                "{}{}",
                text(end..to),
                if to < self.original.len() { "…" } else { "" }
            ),
        })
    }
}

/// The plain text of rendered HTML, without tags. Only for searching, so the
/// few entities maud writes are enough to decode.
#[must_use]
pub fn html_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' ');
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_ignores_case_and_accents() {
        assert_eq!(normalize("Màn Quán"), "man quan");
        assert_eq!(normalize("Mèze ÅLAND"), "meze aland");
        assert_eq!(normalize("慢拳"), "慢拳");
    }

    #[test]
    fn query_words_are_normalized_unique_and_limited() {
        assert_eq!(query_words("  Push  hands push "), vec!["push", "hands"]);
        assert_eq!(query_words("   "), Vec::<String>::new());
        assert_eq!(query_words(&"a ".repeat(50)).len(), 1);
        let many = (0..20)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(query_words(&many).len(), MAX_QUERY_WORDS);
    }

    #[test]
    fn words_match_from_their_start() {
        let text = SearchText::new("A woman practices Tàijíquán, 太极拳, and many more.");
        assert!(text.contains("practic"));
        assert!(text.contains("many") && text.contains("man"));
        assert!(!text.contains("oman"));
        assert!(!text.contains("quan"));
        assert!(text.contains("极拳"));
        assert_eq!(find_word("woman man", "man"), Some(6));
    }

    #[test]
    fn excerpt_keeps_the_original_accents() {
        let text = SearchText::new("Here is my version of the slow set, 慢拳 màn quán.");
        assert!(text.contains("man quan"));
        assert_eq!(
            text.excerpt("man quan"),
            Some(Excerpt {
                before: "Here is my version of the slow set, 慢拳 ".to_string(),
                matched: "màn quán".to_string(),
                after: ".".to_string(),
            })
        );
        assert_eq!(
            text.excerpt("慢拳").map(|e| e.matched),
            Some("慢拳".to_string())
        );
        assert_eq!(text.excerpt("sword"), None);
    }

    #[test]
    fn long_excerpt_is_cut_at_words() {
        let text = SearchText::new(&format!(
            "{} needle {}",
            "word ".repeat(40),
            "word ".repeat(60)
        ));
        let excerpt = text.excerpt("needle").ok_or("no excerpt");
        assert!(excerpt.is_ok());
        if let Ok(excerpt) = excerpt {
            assert!(excerpt.before.starts_with("…word "), "{:?}", excerpt.before);
            assert!(excerpt.after.ends_with(" word…"), "{:?}", excerpt.after);
            assert!(excerpt.before.chars().count() <= CONTEXT_BEFORE + 1);
            assert!(excerpt.after.chars().count() <= CONTEXT_AFTER + 1);
        }
    }

    #[test]
    fn html_text_drops_tags_and_decodes_entities() {
        assert_eq!(
            html_text("<h3>Movements</h3><div>Embrace the Bird&#39;s Tail &amp; more</div>"),
            " Movements  Embrace the Bird's Tail & more "
        );
    }
}
