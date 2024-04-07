use perseus::prelude::*;
use serde::{Deserialize, Serialize};
use sycamore::prelude::*;

use super::posts::{get_posts, Post};

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
        img(class = "index", src = "/.perseus/static/img/2023-to-the-states.jpg", alt = "Tai Chi Wayfarer")
        p { "In this blog I'd like to share my love for Tai Chi. I'm going to share some new and old stories or just some thoughts on Tai Chi. I hope you enjoy!" }
        ul { (links) }
        p(class = "reference") {
            a(href = "/feed.xml", rel = "external") {
                img(class = "rss", src = "/.perseus/static/img/rss.png", alt = "Subscribe to Posts")
            }
        }
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
