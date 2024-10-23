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
            title: "Copenhagen, October 2024".into(),
            url: "october-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 10, 23)
                .unwrap()
                .and_hms_opt(16, 35, 00)
                .unwrap(),
        },
        Post {
            title: "Sandy Island 2024".into(),
            url: "sandy-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 9, 18)
                .unwrap()
                .and_hms_opt(15, 15, 00)
                .unwrap(),
        },
        Post {
            title: "Lapland and Mera 2024".into(),
            url: "june-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 6, 20)
                .unwrap()
                .and_hms_opt(12, 21, 00)
                .unwrap(),
        },
        Post {
            title: "Etna Camp 2024".into(),
            url: "etna-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 5, 22)
                .unwrap()
                .and_hms_opt(12, 02, 00)
                .unwrap(),
        },
        Post {
            title: "Mèze and Mariehamn, March 2024".into(),
            url: "march-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 4, 4)
                .unwrap()
                .and_hms_opt(18, 22, 00)
                .unwrap(),
        },
        Post {
            title: "Helsinki and Palermo, February 2024".into(),
            url: "february-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 2, 29)
                .unwrap()
                .and_hms_opt(21, 05, 00)
                .unwrap(),
        },
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
