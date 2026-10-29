use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderValue, Response, StatusCode, header},
    response::{IntoResponse, Redirect},
    routing::get,
};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use rss::{ChannelBuilder, Item};
use std::sync::Arc;
use tower_http::{
    compression::{
        CompressionLayer, DefaultPredicate,
        predicate::{NotForContentType, Predicate},
    },
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
    trace,
    trace::TraceLayer,
};
use tower_livereload::LiveReloadLayer;
use tracing::Level;
use tracing_subscriber::EnvFilter;

use taichiwayfarer::assets;
use taichiwayfarer::data::get_extra_markup;
use taichiwayfarer::posts::{PostLink, PostRepository, SITE_URL};

#[derive(Clone)]
struct AppState {
    posts: Arc<PostRepository>,
}

const DEFAULT_IMAGE: &str = "/static/img/2023-to-the-states.jpg";

/// Page details for the document head, search engines and link previews.
struct PageMeta<'a> {
    title: &'a str,
    description: &'a str,
    /// Canonical path of the page, or `None` for error pages.
    path: Option<&'a str>,
    image: Option<&'a str>,
    og_type: &'a str,
}

fn absolute_url(path: &str) -> String {
    if path.starts_with("http") {
        path.to_string()
    } else {
        format!("{SITE_URL}{path}")
    }
}

fn layout(meta: &PageMeta, content: &Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { (format!("{} ☯ Tai Chi Wayfarer", meta.title)) }
                meta http-equiv="X-UA-Compatible" content="IE=edge";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="description" content=(meta.description);
                @if let Some(path) = meta.path {
                    link rel="canonical" href=(absolute_url(path));
                    meta property="og:url" content=(absolute_url(path));
                }
                meta property="og:site_name" content="Tai Chi Wayfarer";
                meta property="og:type" content=(meta.og_type);
                meta property="og:title" content=(meta.title);
                meta property="og:description" content=(meta.description);
                meta property="og:image" content=(absolute_url(meta.image.unwrap_or(DEFAULT_IMAGE)));
                meta name="twitter:card" content="summary_large_image";
                link href=(assets::url("styles.css")) rel="stylesheet";
                link rel="alternate" type="application/rss+xml" title="Subscribe to Posts" href="/feed.xml";
            }
            body {
                h1 { a href="/" { "Tai Chi Wayfarer" } }
                div .support-me {
                    a .kofi-button href="https://ko-fi.com/D1D61VF42Q" title="Support me on ko-fi.com" target="_blank" rel="noopener" {
                        img src="/static/img/kofi-cup.png" alt="Support me on Ko-fi";
                    }
                }
                nav {
                    ul {
                        li { a href="/about" { "About" } }
                        li { a href="/" { "Posts" } }
                    }
                }
                (content)
            }
        }
    }
}

async fn index(State(state): State<AppState>) -> Markup {
    layout(
        &PageMeta {
            title: "Posts",
            description: "In this blog, I share my love of Tai Chi. I hope you enjoy!",
            path: Some("/"),
            image: None,
            og_type: "website",
        },
        &html! {
            h2 { "Posts" }
            blockquote { "The journey of a thousand miles begins with one step." }
            blockquote { " - Lao Tzu" }
            img .index src=(DEFAULT_IMAGE) alt="Tai Chi Wayfarer";
            p { "In this blog, I share my love of Tai Chi. I hope you enjoy!" }
            p {
                "If you'd like to hear about upcoming camps and workshops or when I publish new posts, "
                a href="mailto:taichiwayfarer@gmail.com?subject=Subscribe&body=Please tell me something about yourself. Who is your teacher, or what kind of content are you interested in?" { "send me an email" }
                ", and I will share the flyers and my updates with you."
            }
            ul .post-list {
                @for post in &state.posts.post_list {
                    li {
                        div  {
                            div {
                                a href=(format!("/post/{}", post.url)) { (post.title) }
                            }
                            div .date {
                                (post.published_at.format("%B %e, %Y"))
                            }
                        }
                    }
                }
            }
            p .reference {
                a href="/feed.xml" rel="external" {
                    img .rss src="/static/img/rss.png" alt="Subscribe to Posts";
                }
            }
        },
    )
}

async fn feed(State(state): State<AppState>) -> impl IntoResponse {
    let items: Vec<Item> = state.posts.post_list.iter().map(Item::from).collect();
    let pub_date = items.first().and_then(|item| item.pub_date.clone());
    let channel = ChannelBuilder::default()
        .title("Tai Chi Wayfarer")
        .link(SITE_URL)
        .description("RSS feed of Tai Chi Wayfarer posts.")
        .pub_date(pub_date)
        .items(items)
        .build();
    (
        [(header::CONTENT_TYPE, "text/xml; charset=utf-8")],
        channel.to_string(),
    )
}

async fn about() -> Markup {
    layout(
        &PageMeta {
            title: "About",
            description: "Hi, my name is Timo. I've been practicing Tai Chi Chuan since 2002.",
            path: Some("/about"),
            image: Some("/static/img/profile.jpg"),
            og_type: "profile",
        },
        &html! {
            article {
                h2 { "About" }
                blockquote { "Be still like a mountain and flow like a great river." }
                blockquote { " - Lao Tzu" }
                img .index src="/static/img/profile.jpg" alt="Profile picture";
                p { "Hi, my name is Timo. I've been practicing Tai Chi Chuan since 2002. Having this art in my life makes me feel so lucky, happy and grateful." }
                p {
                    "I practice Yang-style Tai Chi in the "
                    a href="http://tungtaichi.com" { "Tung school" }
                    ". The first time I met Grand Master Tung Kai Ying and his son, Master Tung Chen Wei, was at the Lapland camp in 2005. Since then, I've followed them across Europe and North America."
                }
                p { "Tai Chi has changed my life in more ways than I could ever have dreamed. It has given me a new family of friends and has taken me further in my travels than I ever could have imagined." }
            }
        },
    )
}

fn reference_link(link: Option<&PostLink>, text: &str) -> Markup {
    match link {
        None => html! {},
        Some(x) => html! {
            p .reference {
                (text)
                a href=(x.url) { (x.title) }
            }
        },
    }
}

async fn post(Path(url): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    match state.posts.post_map.get(&url) {
        Some(page) => {
            let extra_markup = get_extra_markup(&page.post.url);
            let path = format!("/post/{}", page.post.url);
            layout(
                &PageMeta {
                    title: &page.post.title,
                    description: &page.description,
                    path: Some(&path),
                    image: page.image.as_deref(),
                    og_type: "article",
                },
                &html! {
                    h2 { (page.post.title) }
                    p .date {
                        "Published "
                        (page.post.published_at.format("%B %e, %Y"))
                    }
                    article {
                        (PreEscaped(&page.content))
                        (extra_markup)
                    }
                    (reference_link(page.next_link.as_ref(), "Read the next story? "))
                    (reference_link(page.previous_link.as_ref(), "Read the previous story? "))
                },
            )
            .into_response()
        }
        None => handler_404().await.into_response(),
    }
}

async fn post_redirect(
    Path(url): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if state.posts.post_map.contains_key(&url) {
        Redirect::permanent(&format!("/post/{url}")).into_response()
    } else {
        handler_404().await.into_response()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env before tracing so that RUST_LOG from it applies; variables
    // already set in the environment take precedence over the file.
    let dotenv = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("taichiwayfarer=debug,tower_http=info"))
                .unwrap_or_default(),
        )
        .init();
    match dotenv {
        Ok(path) => tracing::info!(path = %path.display(), "Loaded environment file"),
        Err(error) if error.not_found() => {}
        Err(error) => {
            tracing::error!(%error, "Failed to load environment file");
            return Err(error.into());
        }
    }

    let post_dir = std::env::var("POST_DIR").unwrap_or("posts".to_string());
    let static_dir = std::env::var("STATIC_DIR").unwrap_or("dist".to_string());
    let static_dir = std::path::Path::new(&static_dir);
    assets::init(static_dir);
    let posts = PostRepository::new(std::path::Path::new(&post_dir)).map_err(|error| {
        tracing::error!(%error, "Failed to load posts");
        error
    })?;
    let state = AppState {
        posts: Arc::new(posts),
    };

    // Set but empty (`DEV=`) also counts as on, as used in the Justfile.
    let dev = std::env::var("DEV").is_ok_and(|value| !matches!(value.as_str(), "0" | "false"));
    // Static files are not fingerprinted, so keep the cache short enough for
    // changes to show up within a day. Errors are not cached.
    let cache_control = HeaderValue::from_static(if dev {
        "no-cache"
    } else {
        "public, max-age=86400"
    });
    let static_files = Router::new()
        .nest_service("/static", ServeDir::new(static_dir))
        .route_service(
            "/favicon.ico",
            ServeFile::new(static_dir.join("favicon.ico")),
        )
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            move |response: &Response<_>| {
                response
                    .status()
                    .is_success()
                    .then(|| cache_control.clone())
            },
        ));

    let app = Router::new()
        .merge(static_files)
        .route("/", get(index))
        .route("/feed.xml", get(feed))
        .route("/about", get(about))
        .route("/post/{url}", get(post))
        // Support also directory style urls
        .route("/about/", get(async || Redirect::permanent("/about")))
        .route("/post/{url}/", get(post_redirect))
        .fallback(handler_404)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state);
    let app = if dev {
        app.layer(LiveReloadLayer::new())
    } else {
        app
    };
    // Outermost, so that live reload can still inject into uncompressed HTML.
    // Images are already skipped by the default predicate, and so are fonts here.
    let app = app
        .layer(CompressionLayer::new().compress_when(
            DefaultPredicate::new().and(NotForContentType::const_new("font/woff2")),
        ));
    let port = std::env::var("PORT").unwrap_or("3000".to_string());
    let bind_addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .map_err(|error| {
            tracing::error!(address = %bind_addr, %error, "Failed to bind HTTP listener");
            error
        })?;
    let local_addr = listener.local_addr().map_err(|error| {
        tracing::error!(address = %bind_addr, %error, "Could not determine listener address");
        error
    })?;
    tracing::info!(address = %local_addr, "HTTP server listening");
    if let Err(error) = axum::serve(listener, app.into_make_service()).await {
        tracing::error!(address = %local_addr, %error, "HTTP server failed");
        return Err(error.into());
    }
    tracing::info!(address = %local_addr, "HTTP server stopped");
    Ok(())
}

async fn handler_404() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        layout(
            &PageMeta {
                title: "Not Found",
                description: "The page you are looking for does not exist.",
                path: None,
                image: None,
                og_type: "website",
            },
            &html! {
                h2 { "Not Found" }
                p {
                    "The page you are looking for does not exist. Try "
                    a href="/" { "posts" }
                    " instead."
                }
            },
        ),
    )
}
