use chrono::NaiveDate;
use perseus::prelude::*;
use serde::{Deserialize, Serialize};
use sycamore::prelude::*;

#[derive(Serialize, Deserialize, Clone, ReactiveState)]
pub struct Post {
    pub title: String,
    pub url: String,
    pub published_at: NaiveDate,
}

#[derive(Serialize, Deserialize, Clone, ReactiveState)]
#[rx(alias = "PostsPageRx")]
struct PostsPage {
    posts: Vec<Post>,
}

fn index_page<'a, G: Html>(cx: BoundedScope<'_, 'a>, state: &'a PostsPageRx) -> View<G> {
    let links = state
        .posts
        .get()
        .as_ref()
        .clone()
        .into_iter()
        .fold(vec![], |mut acc, post| {
            acc.push(view! { cx,
                li {
                    p {
                        (format!("{}", post.published_at.format("%B %e, %Y")))
                        " "
                        a(href = format!("/post/{}", post.url)) { (format!("{}", post.title)) }
                    }
                }
            });
            acc
        });
    let links = View::new_fragment(links);
    view! { cx,
        h2 { "Posts" }
        p { "In this blog I'd like to share my love for Tai Chi. I'm going to share some new and old stories or just some thoughts on Tai Chi. I hope you enjoy!" }
        ul { (links) }
    }
}

#[engine_only_fn]
fn head(cx: Scope) -> View<SsrNode> {
    view! { cx,
        title { "Posts - Tai Chi Wayfarer" }
    }
}

#[engine_only_fn]
async fn get_build_state(_: StateGeneratorInfo<()>) -> PostsPage {
    let posts = get_posts();
    PostsPage { posts }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::build("index")
        .build_state_fn(get_build_state)
        .view_with_state(index_page)
        .head(head)
        .build()
}

pub fn get_posts() -> Vec<Post> {
    // Note: Last post is first, so prepend new stories!
    vec![
        Post {
            title: "Sandy Island 2023".into(),
            url: "sandy-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 12, 12).unwrap(),
        },
        Post {
            title: "European Tour 2023".into(),
            url: "november-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 11, 10).unwrap(),
        },
        Post {
            title: "Bosco Falconeria, Harvest of Catarratto".into(),
            url: "bosco-harvest-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 10, 27).unwrap(),
        },
    ]
}
