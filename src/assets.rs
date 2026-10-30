use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};
use std::path::Path;
use std::sync::OnceLock;

/// Static files whose URLs carry a content hash, so browsers fetch them
/// again whenever they change.
const VERSIONED_FILES: &[&str] = &["styles.css", "embed-video.js"];

/// Latin subsets of the fonts every page shows at once (title and text), which
/// pages ask the browser to fetch together with the stylesheet. They must match
/// the `@font-face` URLs in input.css.
pub const PRELOADED_FONTS: &[&str] = &[
    "fonts/shadows-into-light-latin.woff2",
    "fonts/inter-latin.woff2",
];

/// Other static files the page layout links to. Images in posts are checked
/// when the posts are loaded.
const LINKED_FILES: &[&str] = &[
    "favicon.ico",
    "img/kofi-cup.png",
    "img/rss.png",
    "img/2023-to-the-states.jpg",
    "img/profile.jpg",
];

static VERSIONS: OnceLock<HashMap<&'static str, String>> = OnceLock::new();

/// Width and height of the images among `LINKED_FILES`.
static IMAGE_SIZES: OnceLock<HashMap<&'static str, (usize, usize)>> = OnceLock::new();

fn content_hash(bytes: &[u8]) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    format!("{:016x}", hasher.finish())
}

/// The files pages link to that are not in `static_dir`.
#[must_use]
pub fn missing(static_dir: &Path) -> Vec<&'static str> {
    VERSIONED_FILES
        .iter()
        .chain(PRELOADED_FONTS)
        .chain(LINKED_FILES)
        .copied()
        .filter(|name| !static_dir.join(name).is_file())
        .collect()
}

/// Hashes the versioned files in `static_dir` and reads the sizes of the
/// linked images. Only the first call has an effect.
/// A missing file is skipped: its URL is left unversioned, or its size unknown.
pub fn init(static_dir: &Path) {
    VERSIONS.get_or_init(|| {
        VERSIONED_FILES
            .iter()
            .filter_map(|&name| {
                let bytes = std::fs::read(static_dir.join(name)).ok()?;
                Some((name, content_hash(&bytes)))
            })
            .collect()
    });
    IMAGE_SIZES.get_or_init(|| {
        LINKED_FILES
            .iter()
            .filter_map(|&name| {
                let size = imagesize::size(static_dir.join(name)).ok()?;
                Some((name, (size.width, size.height)))
            })
            .collect()
    });
}

/// Width and height of a linked image, e.g. `img/profile.jpg`, as read by `init`.
#[must_use]
pub fn image_size(name: &str) -> Option<(usize, usize)> {
    IMAGE_SIZES.get()?.get(name).copied()
}

/// Whether `version` is the current content hash of the static file `name`.
#[must_use]
pub fn is_current(name: &str, version: &str) -> bool {
    VERSIONS
        .get()
        .and_then(|versions| versions.get(name))
        .is_some_and(|current| current == version)
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

    #[test]
    fn missing_lists_files_not_in_static_dir() {
        let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist");
        // styles.css is built by Tailwind, so it may not exist yet.
        assert!(
            missing(&dist).iter().all(|&name| name == "styles.css"),
            "{:?}",
            missing(&dist)
        );
        let all = VERSIONED_FILES.len() + PRELOADED_FONTS.len() + LINKED_FILES.len();
        assert_eq!(missing(Path::new("no-such-dir")).len(), all);
    }

    #[test]
    fn images_are_formats_the_build_shrinks() -> std::io::Result<()> {
        // nix/package.nix resizes these and removes their metadata, such as
        // GPS locations. Others would be served as they are.
        let img = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist/img");
        let mut others = Vec::new();
        for entry in std::fs::read_dir(img)? {
            let path = entry?.path();
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_lowercase);
            if !matches!(extension.as_deref(), Some("jpg" | "jpeg" | "png")) {
                others.push(path);
            }
        }
        assert!(
            others.is_empty(),
            "convert these to JPEG or PNG: {others:?}"
        );
        Ok(())
    }

    #[test]
    fn preloaded_fonts_match_stylesheet() {
        let css = include_str!("../input.css");
        for font in PRELOADED_FONTS {
            assert!(css.contains(&format!("url({font})")), "{font}");
        }
    }
}
