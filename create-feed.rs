#!/usr/bin/env rust-script
//! Dependencies can be specified in the script file itself as follows:
//!
//! ```cargo
//! [dependencies]
//! rss = "2.0.6"
//! serde = { version = "1", features = [ "derive" ] }
//! chrono = { version = "0.4.31", features = [ "serde" ] }
//! ```

use rss::{ChannelBuilder, Item};
use std::collections::BTreeMap;

include!("src/templates/posts.rs");

impl From<&Post> for Item {
    fn from(post: &Post) -> Self {
        Item {
            title: Some(post.title.clone()),
            link: Some(format!("https://www.taichiwayfarer.com/post/{}", post.url)),
            description: None,
            author: None,
            categories: Vec::new(),
            comments: None,
            enclosure: None,
            guid: None,
            pub_date: Some(post.published_at.format("%a, %d %b %Y %T EET").to_string()),
            source: None,
            content: None,
            extensions: BTreeMap::new(),
            itunes_ext: None,
            dublin_core_ext: None,
        }
    }
}

fn main() {
    let items: Vec<Item> = get_posts().iter().map(Item::from).collect();
    let pub_date = if items.len() > 0 {
        items[0].pub_date.clone()
    } else {
        None
    };
    let channel = ChannelBuilder::default()
        .title("Tai Chi Wayfarer")
        .link("https://www.taichiwayfarer.com")
        .description("And RSS feed of Tai Chi Wayfarer posts.")
        .pub_date(pub_date)
        .items(items)
        .build();
    println!("{}", channel.to_string());
}
