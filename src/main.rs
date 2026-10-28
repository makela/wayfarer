use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use rss::{ChannelBuilder, Item};
use std::sync::Arc;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace,
    trace::TraceLayer,
};
use tower_livereload::LiveReloadLayer;
use tracing::Level;
use tracing_subscriber::EnvFilter;

mod data;
use data::get_extra_markup;
mod posts;
use posts::{PostLink, PostRepository};

#[derive(Clone)]
struct AppState {
    posts: Arc<PostRepository>,
}

fn layout(title: &str, content: &Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
        head {
            title { (format!("{title} ☯ Tai Chi Wayfarer")) }
            meta charset="utf-8";
            meta http-equiv="X-UA-Compatible" content="IE=edge";
            meta name="viewport" content="width=device-width, initial-scale=1";
            meta name="description" content="Tai Chi Wayfarer Blog";
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="true";
            link href="https://fonts.googleapis.com/css2?family=Shadows+Into+Light&family=Gloria+Hallelujah&family=Inter&display=swap" rel="stylesheet";
            link href="/static/styles.css" rel="stylesheet";
            link rel="alternate" type="application/rss+xml" title="Subscribe to Posts" href="/feed.xml";
        }
        body {
            h1 { a href="/" { "Tai Chi Wayfarer" } }
            div .support-me {
                span {
                    (PreEscaped("
<script type='text/javascript' src='https://storage.ko-fi.com/cdn/widget/Widget_2.js'></script><script type='text/javascript'>kofiwidget2.init('', '#1E293B', 'D1D61VF42Q');kofiwidget2.draw();</script>
                    "))
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
        "Posts",
        &html! {
            h2 { "Posts" }
            blockquote { "The journey of a thousand miles begins with one step." }
            blockquote { " - Lao Tzu" }
            img .index src="/static/img/2023-to-the-states.jpg" alt="Tai Chi Wayfarer";
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
                                (format!("{}", post.published_at.format("%B %e, %Y")))
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

#[expect(clippy::unwrap_used)]
async fn feed(State(state): State<AppState>) -> impl IntoResponse {
    let items: Vec<Item> = state.posts.post_list.iter().map(Item::from).collect();
    let pub_date = if items.is_empty() {
        None
    } else {
        items[0].pub_date.clone()
    };
    let channel = ChannelBuilder::default()
        .title("Tai Chi Wayfarer")
        .link("https://www.taichiwayfarer.com")
        .description("RSS feed of Tai Chi Wayfarer posts.")
        .pub_date(pub_date)
        .items(items)
        .build();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "text/xml; charset=utf-8".parse().unwrap(),
    );
    (headers, channel.to_string()).into_response()
}

async fn about() -> Markup {
    layout(
        "About",
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

fn reference_link(link: &Option<PostLink>, text: &str) -> Markup {
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
            layout(
                &page.post.title,
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
                    (reference_link(&page.next_link, "Read the next story? "))
                    (reference_link(&page.previous_link, "Read the previous story? "))
                },
            )
            .into_response()
        }
        None => handler_404().await.into_response(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("taichiwayfarer=debug,tower_http=info"))
                .unwrap_or_default(),
        )
        .init();

    let state = AppState {
        posts: Arc::new(PostRepository::new()),
    };

    let app = Router::new()
        .nest_service("/static", ServeDir::new("dist"))
        .route("/", get(index))
        .route("/feed.xml", get(feed))
        .route("/about", get(about))
        .route("/post/{url}", get(post))
        // Support also directory style urls
        .route("/about/", get(async || Redirect::permanent("/about")))
        .route(
            "/post/{url}/",
            get(async |Path(url): Path<String>| Redirect::permanent(&format!("/post/{url}"))),
        )
        .route_service("/favicon.ico", ServeFile::new("dist/favicon.ico"))
        .fallback(handler_404)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state);
    let app = if std::env::var("DEV").is_ok() {
        app.layer(LiveReloadLayer::new())
    } else {
        app
    };
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
            "Not Found",
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
