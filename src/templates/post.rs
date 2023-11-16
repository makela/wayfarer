use super::index::get_posts;
use chrono::NaiveDate;
use perseus::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::{Error, ErrorKind};
use std::path::Path;
use sycamore::prelude::*;

#[derive(Serialize, Deserialize, Clone)]
struct PostLink {
    title: String,
    url: String,
}

#[derive(Serialize, Deserialize, Clone, ReactiveState)]
#[rx(alias = "PostStateRx")]
struct PostState {
    title: String,
    content: String,
    published_at: NaiveDate,
    previous: Option<PostLink>,
    next: Option<PostLink>,
}

fn reference_link<G: Html>(cx: Scope, link: Option<PostLink>, text: String) -> View<G> {
    match link {
        None => view! { cx, },
        Some(x) => view! { cx,
            p(class = "reference") {
                (text)
                a(href = x.url) { (x.title) }
            }
        },
    }
}

#[auto_scope]
fn post_page<G: Html>(cx: Scope, state: &PostStateRx) -> View<G> {
    let previous: View<G> = reference_link(
        cx,
        state.previous.get().as_ref().clone(),
        "Read the previous story? ".into(),
    );
    let next: View<G> = reference_link(
        cx,
        state.next.get().as_ref().clone(),
        "Read the next story? ".into(),
    );
    view! { cx,
        h2 { (state.title.get()) }
        p(class = "date") {
            "Published "
            (format!("{}", state.published_at.get().format("%B %e, %Y")))
        }
        article(dangerously_set_inner_html = &state.content.get())
        (next)
        (previous)
    }
}

#[engine_only_fn]
async fn get_build_paths() -> BuildPaths {
    let posts = get_posts();
    let paths = posts.iter().map(|post| post.url.clone()).collect();
    BuildPaths {
        paths,
        extra: ().into(),
    }
}

#[engine_only_fn]
async fn get_build_state(info: StateGeneratorInfo<()>) -> Result<PostState, BlamedError<Error>> {
    let posts = get_posts();
    let index = posts.iter().position(|post| post.url == info.path).unwrap();
    match posts.get(index) {
        None => Err(Error::new(ErrorKind::NotFound, "Post not found").into()),
        Some(post) => {
            let file = format!("./posts/{}.md", post.url);
            let content = markdown::file_to_html(Path::new(&file)).unwrap();
            // Negative index seems to fail on wasm, therefore have to text it manually.
            let next = if index > 0 {
                match posts.get(index - 1) {
                    Some(post) => Some(PostLink {
                        title: post.title.clone(),
                        url: format!("/post/{}", post.url),
                    }),
                    None => None,
                }
            } else {
                None
            };
            let previous = match posts.get(index + 1) {
                Some(post) => Some(PostLink {
                    title: post.title.clone(),
                    url: format!("/post/{}", post.url),
                }),
                None => None,
            };
            Ok(PostState {
                title: post.title.clone(),
                published_at: post.published_at,
                content,
                previous,
                next,
            })
        }
    }
}

#[engine_only_fn]
fn head(cx: Scope, state: PostState) -> View<SsrNode> {
    view! { cx,
        title { (format!("{} - Tai Chi Wayfarer", state.title)) }
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::build("post")
        .build_paths_fn(get_build_paths)
        .build_state_fn(get_build_state)
        .view_with_state(post_page)
        .head_with_state(head)
        .build()
}
