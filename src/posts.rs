use chrono::{NaiveDateTime, TimeZone};
use rss::Item;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

#[derive(Clone)]
pub struct Post {
    pub title: String,
    pub url: String,
    pub published_at: NaiveDateTime,
}

impl From<&Post> for Item {
    fn from(post: &Post) -> Self {
        let pub_date = chrono_tz::Europe::Helsinki
            .from_local_datetime(&post.published_at)
            .earliest()
            .map(|date| date.to_rfc2822());

        Item {
            title: Some(post.title.clone()),
            link: Some(format!("https://www.taichiwayfarer.com/post/{}", post.url)),
            description: None,
            author: None,
            categories: Vec::new(),
            comments: None,
            enclosure: None,
            guid: None,
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

/// Published posts as (title, url, published at in Helsinki time "YYYY-MM-DD HH:MM").
/// Note: The last post is first, so prepend new stories!
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
                .map_err(|error| format!("invalid date {published_at:?} for post {url}: {error}"))?;
            Ok(Post {
                title: title.into(),
                url: url.into(),
                published_at,
            })
        })
        .collect()
}

#[derive(Clone)]
pub struct PostPage {
    pub post: Post,
    pub content: String,
    pub previous_link: Option<PostLink>,
    pub next_link: Option<PostLink>,
}

#[derive(Clone)]
pub struct PostRepository {
    pub post_list: Vec<Post>,
    pub post_map: HashMap<String, PostPage>,
}

impl PostRepository {
    pub fn new(post_dir: &Path) -> Result<Self, String> {
        let post_list = get_posts()?;
        let mut post_map: HashMap<String, PostPage> = HashMap::new();
        let mut next_link: Option<PostLink> = None;
        for post in post_list.clone() {
            if let Some(next) = next_link.clone()
            && let Some(next_page) = post_map.get_mut(&next.url) {
                next_page.previous_link = Some(PostLink {
                    title: post.title.clone(),
                    url: post.url.clone(),
                });
            }
            let file = post_dir.join(format!("{}.md", post.url));
            let input = fs::read_to_string(&file)
                .map_err(|error| format!("{}: {error}", file.display()))?;
            let content = markdown::to_html(&input);
            post_map.insert(
                post.url.clone(),
                PostPage {
                    post: post.clone(),
                    content,
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
        let post_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("posts");
        let result = PostRepository::new(&post_dir);
        assert!(result.is_ok(), "{:?}", result.err());
    }

    #[test]
    fn every_post_date_is_valid() {
        let result = get_posts();
        assert!(result.is_ok(), "{:?}", result.err());
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
