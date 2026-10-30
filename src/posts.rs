use chrono::{NaiveDateTime, TimeZone};
use markdown::mdast::Node;
use rss::{Guid, Item};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;
use std::fs;
use std::path::Path;

/// Page descriptions are cut to about the length search engines show.
const DESCRIPTION_LENGTH: usize = 160;

/// Path of the page of the post named `url`, e.g. `/post/the-slow-set`.
/// The one place post links are formatted; the routes in main.rs match it.
#[must_use]
pub fn post_path(url: &str) -> String {
    format!("/post/{url}")
}

#[derive(Clone)]
pub struct Post {
    pub title: String,
    pub url: String,
    pub published_at: NaiveDateTime,
}

impl Post {
    /// Path of the post's page, see [`post_path`].
    #[must_use]
    pub fn path(&self) -> String {
        post_path(&self.url)
    }

    /// The post as an RSS item, linking to it under `site_url`.
    #[must_use]
    pub fn to_item(&self, site_url: &str) -> Item {
        let pub_date = chrono_tz::Europe::Helsinki
            .from_local_datetime(&self.published_at)
            .earliest()
            .map(|date| date.to_rfc2822());

        let link = format!("{site_url}{}", self.path());

        Item {
            title: Some(self.title.clone()),
            link: Some(link.clone()),
            description: None,
            author: None,
            categories: Vec::new(),
            comments: None,
            enclosure: None,
            guid: Some(Guid {
                value: link,
                permalink: true,
            }),
            pub_date,
            source: None,
            content: None,
            extensions: BTreeMap::new(),
            itunes_ext: None,
            dublin_core_ext: None,
        }
    }
}

#[derive(Clone)]
pub struct PostLink {
    pub title: String,
    pub url: String,
}

impl PostLink {
    /// Path of the linked post's page, see [`post_path`].
    #[must_use]
    pub fn path(&self) -> String {
        post_path(&self.url)
    }
}

/// Published posts as (title, url, published at in Helsinki time "YYYY-MM-DD HH:MM").
/// Note: The last post is first, so prepend new stories!
#[rustfmt::skip]
const POSTS: &[(&str, &str, &str)] = &[
    // ("The Sword Set", "the-sword-set", "2026-08-08 08:08"),
    // ("Chinese Glossary", "chinese-glossary", "2026-08-08 08:08"),
    ("Don't Think — Just Do", "dont-think-just-do", "2026-10-24 15:28"),
    ("Learn by Watching", "learn-by-watching", "2026-10-08 11:41"),
    ("The Slow Set", "the-slow-set", "2026-08-18 19:52"),
    ("Come to the Center", "june-2026", "2026-07-05 22:26"),
    ("Be Aware of Your Body", "etna-2026", "2026-05-29 13:17"),
    ("Remember by Yourself", "april-2026", "2026-04-30 16:34"),
    ("Take Your Time", "march-2026", "2026-03-31 16:21"),
    ("Be a Shadow", "february-2026", "2026-02-28 12:32"),
    ("Do Your Own Way", "january-2026", "2026-02-11 14:27"),
    ("Use Your Mind", "december-2025", "2025-12-14 11:44"),
    ("Practice On Your Own", "november-2025", "2025-12-03 18:28"),
    ("Don't Stop", "october-2025", "2025-10-20 17:55"),
    ("Do Bigger", "sandy-2025", "2025-09-23 14:05"),
    ("Always Practice", "la-honda-2025", "2025-09-13 18:08"),
    ("Together", "mera-2025", "2025-07-02 10:44"),
    ("Don't Do Anything Extra", "lapland-2025", "2025-06-25 20:25"),
    ("Raise Your Spirit", "may-2025", "2025-06-20 14:22"),
    ("Don't Teach Too Much", "april-2025", "2025-04-17 14:31"),
    ("Pay Attention", "march-2025", "2025-04-08 09:45"),
    ("Match Your Energy", "february-2025", "2025-02-13 09:45"),
    ("Know Your Space", "january-2025", "2025-01-31 17:32"),
    ("It's all about the Body", "december-2024", "2024-12-02 17:46"),
    ("Play Tai Chi", "november-2024", "2024-11-30 22:02"),
    ("Copenhagen, October 2024", "october-2024", "2024-10-23 17:05"),
    ("Sandy Island 2024", "sandy-2024", "2024-09-18 15:15"),
    ("Lapland and Mera 2024", "june-2024", "2024-06-20 12:21"),
    ("Etna Camp 2024", "etna-2024", "2024-05-22 12:02"),
    ("Mèze and Mariehamn, March 2024", "march-2024", "2024-04-04 18:22"),
    ("Helsinki and Palermo, February 2024", "february-2024", "2024-02-29 21:05"),
    ("Modesto and Vermont 2023", "modesto-and-vermont-2023", "2024-01-06 14:45"),
    ("Sandy Island 2023", "sandy-2023", "2023-12-12 15:00"),
    ("European Tour 2023", "november-2023", "2023-11-10 16:05"),
    ("Bosco Falconeria, Harvest of Catarratto", "bosco-harvest-2023", "2023-10-27 17:15"),
];

fn get_posts() -> Result<Vec<Post>, String> {
    POSTS
        .iter()
        .map(|&(title, url, published_at)| {
            let published_at = NaiveDateTime::parse_from_str(published_at, "%Y-%m-%d %H:%M")
                .map_err(|error| {
                    format!("invalid date {published_at:?} for post {url}: {error}")
                })?;
            Ok(Post {
                title: title.into(),
                url: url.into(),
                published_at,
            })
        })
        .collect()
}

/// Collects the plain text of a markdown node, without formatting.
fn plain_text(node: &Node, text: &mut String) {
    match node {
        Node::Text(t) => text.push_str(&t.value),
        Node::InlineCode(code) => text.push_str(&code.value),
        Node::Break(_) => text.push(' '),
        _ => {
            for child in node.children().into_iter().flatten() {
                plain_text(child, text);
            }
        }
    }
}

fn first_image(node: &Node) -> Option<&str> {
    match node {
        Node::Image(image) => Some(&image.url),
        _ => node.children()?.iter().find_map(first_image),
    }
}

/// Shortens text to at most `max` characters, cutting at a word boundary.
fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max - 1).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([' ', ',', '.', ';', ':', '-']))
}

/// Returns the first paragraph as a short description, and the first image.
fn summarize(input: &str) -> Result<(Option<String>, Option<String>), String> {
    let tree = markdown::to_mdast(input, &markdown::ParseOptions::default())
        .map_err(|error| error.to_string())?;
    let description = tree.children().into_iter().flatten().find_map(|node| {
        let Node::Paragraph(_) = node else {
            return None;
        };
        let mut text = String::new();
        plain_text(node, &mut text);
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        (!text.is_empty()).then(|| truncate(&text, DESCRIPTION_LENGTH))
    });
    Ok((description, first_image(&tree).map(str::to_string)))
}

/// Adds the size of each local image (`/static/...`, read from `static_dir`)
/// to its `<img>` tag, so the text doesn't jump while photos load, and lets
/// all but the first image load only when scrolled into view.
fn enhance_images(html: &str, static_dir: &Path) -> Result<String, String> {
    const IMG: &str = "<img src=\"";
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut first = true;
    while let Some(start) = rest.find(IMG) {
        out.push_str(&rest[..start]);
        rest = &rest[start + IMG.len()..];
        let src = &rest[..rest.find('"').ok_or("unterminated <img> src")?];
        out.push_str("<img");
        if let Some(path) = src.strip_prefix("/static/") {
            let file = static_dir.join(path);
            let size = imagesize::size(&file)
                .map_err(|error| format!("image {}: {error}", file.display()))?;
            write!(out, r#" width="{}" height="{}""#, size.width, size.height)
                .map_err(|error| error.to_string())?;
        }
        if !first {
            out.push_str(r#" loading="lazy""#);
        }
        first = false;
        out.push_str(r#" src=""#);
    }
    out.push_str(rest);
    Ok(out)
}

#[derive(Clone)]
pub struct PostPage {
    pub post: Post,
    pub content: String,
    pub description: String,
    pub image: Option<String>,
    pub previous_link: Option<PostLink>,
    pub next_link: Option<PostLink>,
}

#[derive(Clone)]
pub struct PostRepository {
    pub post_list: Vec<Post>,
    pub post_map: HashMap<String, PostPage>,
}

impl PostRepository {
    /// Loads the posts from `post_dir`. Images they refer to under `/static/`
    /// must exist in `static_dir`.
    pub fn new(post_dir: &Path, static_dir: &Path) -> Result<Self, String> {
        let post_list = get_posts()?;
        let mut post_map: HashMap<String, PostPage> = HashMap::new();
        let mut next_link: Option<PostLink> = None;
        for post in post_list.clone() {
            if let Some(next) = next_link.clone()
                && let Some(next_page) = post_map.get_mut(&next.url)
            {
                next_page.previous_link = Some(PostLink {
                    title: post.title.clone(),
                    url: post.url.clone(),
                });
            }
            let file = post_dir.join(format!("{}.md", post.url));
            let input = fs::read_to_string(&file)
                .map_err(|error| format!("{}: {error}", file.display()))?;
            let content = enhance_images(&markdown::to_html(&input), static_dir)
                .map_err(|error| format!("{}: {error}", file.display()))?;
            let (description, image) =
                summarize(&input).map_err(|error| format!("{}: {error}", file.display()))?;
            post_map.insert(
                post.url.clone(),
                PostPage {
                    post: post.clone(),
                    content,
                    description: description.unwrap_or_else(|| post.title.clone()),
                    image,
                    next_link,
                    previous_link: None,
                },
            );
            next_link = Some(PostLink {
                title: post.title.clone(),
                url: post.url.clone(),
            });
        }
        Ok(PostRepository {
            post_list,
            post_map,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_post_file_loads() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let result = PostRepository::new(&root.join("posts"), &root.join("dist"));
        assert!(result.is_ok(), "{:?}", result.err());
    }

    #[test]
    fn every_post_date_is_valid() -> Result<(), String> {
        for post in get_posts()? {
            // Fails for times skipped when clocks go forward in Helsinki
            assert!(
                post.to_item("https://example.com").pub_date.is_some(),
                "{} has no valid Helsinki time",
                post.url
            );
        }
        Ok(())
    }

    #[test]
    fn summary_uses_first_paragraph_and_image() -> Result<(), String> {
        let input = "![Group](/static/img/group.jpg)\n\nWe *practiced* the `slow set`.\n\nMore.";
        let (description, image) = summarize(input)?;
        assert_eq!(description.as_deref(), Some("We practiced the slow set."));
        assert_eq!(image.as_deref(), Some("/static/img/group.jpg"));
        Ok(())
    }

    #[test]
    fn images_get_size_and_lazy_loading() -> Result<(), String> {
        let static_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist");
        let html = markdown::to_html(
            "![RSS](/static/img/rss.png)\n\n![Cup](/static/img/kofi-cup.png)\n\n![Ext](https://example.com/a.jpg)",
        );
        let html = enhance_images(&html, &static_dir)?;
        let cup =
            imagesize::size(static_dir.join("img/kofi-cup.png")).map_err(|e| e.to_string())?;
        let rss = imagesize::size(static_dir.join("img/rss.png")).map_err(|e| e.to_string())?;
        assert!(
            html.contains(&format!(
                r#"<img width="{}" height="{}" src="/static/img/rss.png""#,
                rss.width, rss.height
            )),
            "{html}"
        );
        assert!(
            html.contains(&format!(
                r#"<img width="{}" height="{}" loading="lazy" src="/static/img/kofi-cup.png""#,
                cup.width, cup.height
            )),
            "{html}"
        );
        assert!(
            html.contains(r#"<img loading="lazy" src="https://example.com/a.jpg""#),
            "{html}"
        );
        Ok(())
    }

    #[test]
    fn missing_image_is_an_error() {
        let static_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("dist");
        let html = markdown::to_html("![Typo](/static/img/no-such-photo.jpg)");
        assert!(enhance_images(&html, &static_dir).is_err());
    }

    #[test]
    fn long_description_is_cut_at_a_word() {
        let text = "word ".repeat(50);
        let description = truncate(text.trim(), DESCRIPTION_LENGTH);
        assert!(description.chars().count() <= DESCRIPTION_LENGTH);
        assert!(description.ends_with("word…"), "{description}");
    }

    #[test]
    fn post_urls_are_unique() -> Result<(), String> {
        let posts = get_posts()?;
        let mut seen = std::collections::HashSet::new();
        for post in &posts {
            assert!(seen.insert(&post.url), "duplicate post url {}", post.url);
        }
        Ok(())
    }

    #[test]
    fn posts_are_sorted_newest_first() -> Result<(), String> {
        let posts = get_posts()?;
        for pair in posts.windows(2) {
            assert!(
                pair[0].published_at > pair[1].published_at,
                "{} must be newer than {}",
                pair[0].url,
                pair[1].url
            );
        }
        Ok(())
    }
}
