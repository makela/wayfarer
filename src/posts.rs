use chrono::{NaiveDate, NaiveDateTime, TimeZone};
use rss::Item;
use std::collections::{BTreeMap, HashMap};
use std::fs;

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
            .map(|date| date.format("%a, %d %b %Y %T %Z").to_string());

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

#[expect(clippy::too_many_lines)]
#[must_use]
fn get_posts() -> Vec<Post> {
    // Note: The last post is first, so prepend new stories!
    vec![
        /*
        Post {
            title: "The Sword Set".into(),
            url: "the-sword-set".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 8, 8)
                .unwrap_or_default()
                .and_hms_opt(8, 8, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Chinese Glossary".into(),
            url: "chinese-glossary".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 8, 8)
                .unwrap_or_default()
                .and_hms_opt(8, 8, 0)
                .unwrap_or_default(),
        },
        */
        Post {
            title: "Learn by Watching".into(),
            url: "learn-by-watching".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 10, 8)
                .unwrap_or_default()
                .and_hms_opt(11, 41, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "The Slow Set".into(),
            url: "the-slow-set".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 8, 18)
                .unwrap_or_default()
                .and_hms_opt(19, 52, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Come to the Center".into(),
            url: "june-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 7, 5)
                .unwrap_or_default()
                .and_hms_opt(22, 26, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Be Aware of Your Body".into(),
            url: "etna-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 5, 29)
                .unwrap_or_default()
                .and_hms_opt(13, 17, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Remember by Yourself".into(),
            url: "april-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 4, 30)
                .unwrap_or_default()
                .and_hms_opt(16, 34, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Take Your Time".into(),
            url: "march-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 3, 31)
                .unwrap_or_default()
                .and_hms_opt(16, 21, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Be a Shadow".into(),
            url: "february-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 2, 28)
                .unwrap_or_default()
                .and_hms_opt(12, 32, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Do Your Own Way".into(),
            url: "january-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 2, 11)
                .unwrap_or_default()
                .and_hms_opt(14, 27, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Use Your Mind".into(),
            url: "december-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 12, 14)
                .unwrap_or_default()
                .and_hms_opt(11, 44, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Practice On Your Own".into(),
            url: "november-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 12, 3)
                .unwrap_or_default()
                .and_hms_opt(18, 28, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Don't Stop".into(),
            url: "october-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 10, 20)
                .unwrap_or_default()
                .and_hms_opt(17, 55, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Do Bigger".into(),
            url: "sandy-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 9, 23)
                .unwrap_or_default()
                .and_hms_opt(14, 5, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Always Practice".into(),
            url: "la-honda-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 9, 13)
                .unwrap_or_default()
                .and_hms_opt(18, 8, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Together".into(),
            url: "mera-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 7, 2)
                .unwrap_or_default()
                .and_hms_opt(10, 44, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Don't Do Anything Extra".into(),
            url: "lapland-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 6, 25)
                .unwrap_or_default()
                .and_hms_opt(20, 25, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Raise Your Spirit".into(),
            url: "may-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 6, 20)
                .unwrap_or_default()
                .and_hms_opt(14, 22, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Don't Teach Too Much".into(),
            url: "april-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 4, 17)
                .unwrap_or_default()
                .and_hms_opt(14, 31, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Pay Attention".into(),
            url: "march-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 4, 8)
                .unwrap_or_default()
                .and_hms_opt(9, 45, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Match Your Energy".into(),
            url: "february-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 2, 13)
                .unwrap_or_default()
                .and_hms_opt(9, 45, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Know Your Space".into(),
            url: "january-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 1, 31)
                .unwrap_or_default()
                .and_hms_opt(17, 32, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "It's all about the Body".into(),
            url: "december-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 12, 2)
                .unwrap_or_default()
                .and_hms_opt(17, 46, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Play Tai Chi".into(),
            url: "november-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 11, 30)
                .unwrap_or_default()
                .and_hms_opt(22, 2, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Copenhagen, October 2024".into(),
            url: "october-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 10, 23)
                .unwrap_or_default()
                .and_hms_opt(17, 5, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Sandy Island 2024".into(),
            url: "sandy-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 9, 18)
                .unwrap_or_default()
                .and_hms_opt(15, 15, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Lapland and Mera 2024".into(),
            url: "june-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 6, 20)
                .unwrap_or_default()
                .and_hms_opt(12, 21, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Etna Camp 2024".into(),
            url: "etna-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 5, 22)
                .unwrap_or_default()
                .and_hms_opt(12, 2, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Mèze and Mariehamn, March 2024".into(),
            url: "march-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 4, 4)
                .unwrap_or_default()
                .and_hms_opt(18, 22, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Helsinki and Palermo, February 2024".into(),
            url: "february-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 2, 29)
                .unwrap_or_default()
                .and_hms_opt(21, 5, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Modesto and Vermont 2023".into(),
            url: "modesto-and-vermont-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 1, 6)
                .unwrap_or_default()
                .and_hms_opt(14, 45, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Sandy Island 2023".into(),
            url: "sandy-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 12, 12)
                .unwrap_or_default()
                .and_hms_opt(15, 0, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "European Tour 2023".into(),
            url: "november-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 11, 10)
                .unwrap_or_default()
                .and_hms_opt(16, 5, 0)
                .unwrap_or_default(),
        },
        Post {
            title: "Bosco Falconeria, Harvest of Catarratto".into(),
            url: "bosco-harvest-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 10, 27)
                .unwrap_or_default()
                .and_hms_opt(17, 15, 0)
                .unwrap_or_default(),
        },
    ]
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
    pub fn new() -> Self {
        let post_list = get_posts();
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
            let file = format!("./posts/{}.md", post.url);
            let input = fs::read_to_string(std::path::Path::new(&file)).unwrap_or_default();
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
        PostRepository {
            post_list,
            post_map,
        }
    }
}
