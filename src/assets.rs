use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};
use std::path::Path;
use std::sync::OnceLock;

/// Static files whose URLs carry a content hash, so browsers fetch them
/// again whenever they change.
const VERSIONED_FILES: &[&str] = &["styles.css", "embed-video.js"];

static VERSIONS: OnceLock<HashMap<&'static str, String>> = OnceLock::new();

fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

/// Hashes the versioned files in `static_dir`. Call once at startup.
/// A missing file is skipped, and its URL is then left unversioned.
pub fn init(static_dir: &Path) {
    let versions = VERSIONED_FILES
        .iter()
        .filter_map(|&name| {
            let bytes = std::fs::read(static_dir.join(name)).ok()?;
            Some((name, content_hash(&bytes)))
        })
        .collect();
    // Only the first call has an effect.
    let _ = VERSIONS.set(versions);
}

/// URL of a static file, e.g. `/static/styles.css?v=1a2b3c4d5e6f7a8b`.
#[must_use]
pub fn url(name: &str) -> String {
    match VERSIONS.get().and_then(|versions| versions.get(name)) {
        Some(version) => format!("/static/{name}?v={version}"),
        None => format!("/static/{name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_changes_with_content() {
        assert_eq!(content_hash(b"a"), content_hash(b"a"));
        assert_ne!(content_hash(b"a"), content_hash(b"b"));
    }
}
