use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Post {
    pub title: String,
    pub url: String,
    pub published_at: NaiveDateTime,
}

pub fn get_posts() -> Vec<Post> {
    // Note: Last post is first, so prepend new stories!
    vec![
        Post {
            title: "Modesto and Vermont 2023".into(),
            url: "modesto-and-vermont-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 1, 6)
                .unwrap()
                .and_hms_opt(14, 45, 00)
                .unwrap(),
        },
        Post {
            title: "Sandy Island 2023".into(),
            url: "sandy-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 12, 12)
                .unwrap()
                .and_hms_opt(15, 00, 00)
                .unwrap(),
        },
        Post {
            title: "European Tour 2023".into(),
            url: "november-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 11, 10)
                .unwrap()
                .and_hms_opt(16, 05, 00)
                .unwrap(),
        },
        Post {
            title: "Bosco Falconeria, Harvest of Catarratto".into(),
            url: "bosco-harvest-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 10, 27)
                .unwrap()
                .and_hms_opt(17, 15, 00)
                .unwrap(),
        },
    ]
}
