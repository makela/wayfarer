use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use chrono::{NaiveDate, NaiveDateTime};
use maud::{html, Markup, PreEscaped, DOCTYPE};
use rss::{ChannelBuilder, Item};
use std::collections::BTreeMap;
use std::fs;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace,
    trace::TraceLayer,
};
use tower_livereload::LiveReloadLayer;
use tracing::Level;
use tracing_subscriber::EnvFilter;

struct Post {
    title: String,
    url: String,
    published_at: NaiveDateTime,
}

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

struct PostLink {
    title: String,
    url: String,
}

fn get_posts() -> Vec<Post> {
    // Note: Last post is first, so prepend new stories!
    vec![
        Post {
            title: "Be Aware of Your Body".into(),
            url: "etna-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 5, 29)
                .unwrap()
                .and_hms_opt(13, 17, 0)
                .unwrap(),
        },
        Post {
            title: "Remember by Yourself".into(),
            url: "april-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 4, 30)
                .unwrap()
                .and_hms_opt(16, 34, 0)
                .unwrap(),
        },
        Post {
            title: "Take Your Time".into(),
            url: "march-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 3, 31)
                .unwrap()
                .and_hms_opt(16, 21, 0)
                .unwrap(),
        },
        Post {
            title: "Be a Shadow".into(),
            url: "february-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 2, 28)
                .unwrap()
                .and_hms_opt(12, 32, 0)
                .unwrap(),
        },
        Post {
            title: "Do Your Own Way".into(),
            url: "january-2026".into(),
            published_at: NaiveDate::from_ymd_opt(2026, 2, 11)
                .unwrap()
                .and_hms_opt(14, 27, 0)
                .unwrap(),
        },
        Post {
            title: "Use Your Mind".into(),
            url: "december-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 12, 14)
                .unwrap()
                .and_hms_opt(11, 44, 0)
                .unwrap(),
        },
        Post {
            title: "Practice On Your Own".into(),
            url: "november-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 12, 3)
                .unwrap()
                .and_hms_opt(18, 28, 0)
                .unwrap(),
        },
        Post {
            title: "Don't Stop".into(),
            url: "october-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 10, 20)
                .unwrap()
                .and_hms_opt(17, 55, 0)
                .unwrap(),
        },
        Post {
            title: "Do Bigger".into(),
            url: "sandy-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 9, 23)
                .unwrap()
                .and_hms_opt(14, 5, 0)
                .unwrap(),
        },
        Post {
            title: "Always Practice".into(),
            url: "la-honda-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 9, 13)
                .unwrap()
                .and_hms_opt(18, 8, 0)
                .unwrap(),
        },
        Post {
            title: "Together".into(),
            url: "mera-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 7, 2)
                .unwrap()
                .and_hms_opt(10, 44, 0)
                .unwrap(),
        },
        Post {
            title: "Don't Do Anything Extra".into(),
            url: "lapland-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 6, 25)
                .unwrap()
                .and_hms_opt(20, 25, 0)
                .unwrap(),
        },
        Post {
            title: "Raise Your Spirit".into(),
            url: "may-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 6, 20)
                .unwrap()
                .and_hms_opt(14, 22, 0)
                .unwrap(),
        },
        Post {
            title: "Don't Teach Too Much".into(),
            url: "april-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 4, 17)
                .unwrap()
                .and_hms_opt(14, 31, 0)
                .unwrap(),
        },
        Post {
            title: "Pay Attention".into(),
            url: "march-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 4, 8)
                .unwrap()
                .and_hms_opt(9, 45, 0)
                .unwrap(),
        },
        Post {
            title: "Match Your Energy".into(),
            url: "february-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 2, 13)
                .unwrap()
                .and_hms_opt(9, 45, 0)
                .unwrap(),
        },
        Post {
            title: "Know Your Space".into(),
            url: "january-2025".into(),
            published_at: NaiveDate::from_ymd_opt(2025, 1, 31)
                .unwrap()
                .and_hms_opt(17, 32, 0)
                .unwrap(),
        },
        Post {
            title: "It's all about the Body".into(),
            url: "december-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 12, 2)
                .unwrap()
                .and_hms_opt(17, 46, 0)
                .unwrap(),
        },
        Post {
            title: "Play Tai Chi".into(),
            url: "november-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 11, 30)
                .unwrap()
                .and_hms_opt(22, 2, 0)
                .unwrap(),
        },
        Post {
            title: "Copenhagen, October 2024".into(),
            url: "october-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 10, 23)
                .unwrap()
                .and_hms_opt(17, 5, 0)
                .unwrap(),
        },
        Post {
            title: "Sandy Island 2024".into(),
            url: "sandy-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 9, 18)
                .unwrap()
                .and_hms_opt(15, 15, 0)
                .unwrap(),
        },
        Post {
            title: "Lapland and Mera 2024".into(),
            url: "june-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 6, 20)
                .unwrap()
                .and_hms_opt(12, 21, 0)
                .unwrap(),
        },
        Post {
            title: "Etna Camp 2024".into(),
            url: "etna-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 5, 22)
                .unwrap()
                .and_hms_opt(12, 2, 0)
                .unwrap(),
        },
        Post {
            title: "Mèze and Mariehamn, March 2024".into(),
            url: "march-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 4, 4)
                .unwrap()
                .and_hms_opt(18, 22, 0)
                .unwrap(),
        },
        Post {
            title: "Helsinki and Palermo, February 2024".into(),
            url: "february-2024".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 2, 29)
                .unwrap()
                .and_hms_opt(21, 5, 0)
                .unwrap(),
        },
        Post {
            title: "Modesto and Vermont 2023".into(),
            url: "modesto-and-vermont-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2024, 1, 6)
                .unwrap()
                .and_hms_opt(14, 45, 0)
                .unwrap(),
        },
        Post {
            title: "Sandy Island 2023".into(),
            url: "sandy-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 12, 12)
                .unwrap()
                .and_hms_opt(15, 0, 0)
                .unwrap(),
        },
        Post {
            title: "European Tour 2023".into(),
            url: "november-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 11, 10)
                .unwrap()
                .and_hms_opt(16, 5, 0)
                .unwrap(),
        },
        Post {
            title: "Bosco Falconeria, Harvest of Catarratto".into(),
            url: "bosco-harvest-2023".into(),
            published_at: NaiveDate::from_ymd_opt(2023, 10, 27)
                .unwrap()
                .and_hms_opt(17, 15, 0)
                .unwrap(),
        },
    ]
}

fn layout(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        head {
            title { (format!("{} ☯ Tai Chi Wayfarer", title)) }
            meta charset="utf-8";
            meta http-equiv="X-UA-Compatible" content="IE=edge";
            meta name="viewport" content="width=device-width, initial-scale=1";
            link rel="preconnect" href="https://fonts.googleapis.com";
            link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="true";
            link href="https://fonts.googleapis.com/css2?family=Shadows+Into+Light&family=Gloria+Hallelujah&family=Inter&display=swap" rel="stylesheet";
            link href="/static/tailwind.css" rel="stylesheet";
            link rel="alternative" type="application/rss+xml" title="Subscribe to Posts" href="/feed.xml";
        }
        body {
            h1 { a href="/" { "Tai Chi Wayfarer" } }
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

async fn index() -> Markup {
    layout(
        "Posts",
        html! {
            h2 { "Posts" }
            img .index src="/static/img/2023-to-the-states.jpg" alt="Tai Chi Wayfarer";
            p { "In this blog, I'd like to share my love of Tai Chi. I’ll be sharing both new and old stories, as well as some reflections. I hope you enjoy!" }
            ul .post-list {
                @for post in get_posts() {
                    li {
                        div  {
                            div {
                                a href=(format!("/post/{}", post.url)) { (format!("{}", post.title)) }
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

async fn feed() -> impl IntoResponse {
    let items: Vec<Item> = get_posts().iter().map(Item::from).collect();
    let pub_date = if !items.is_empty() {
        items[0].pub_date.clone()
    } else {
        None
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
        "text/xml; charset=utf.8".parse().unwrap(),
    );
    (headers, channel.to_string()).into_response()
}

async fn about() -> Markup {
    layout(
        "About",
        html! {
            article {
                h2 { "About" }
                blockquote { "Be still like a mountain and flow like a river." }
                blockquote { " - Lao Tzu" }
                p { "Hi, I've been practicing Tai Chi Chuan for over two decades. Having this art in my life makes me feel lucky, happy, and grateful." }
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

fn reference_link(link: Option<PostLink>, text: &str) -> Markup {
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

async fn post(Path(url): Path<String>) -> impl IntoResponse {
    let posts = get_posts();
    match posts.iter().position(|post| post.url == url) {
        Some(index) => match posts.get(index) {
            Some(post) => {
                let file = format!("./posts/{}.md", post.url);
                let input = fs::read_to_string(std::path::Path::new(&file)).unwrap();
                let content = markdown::to_html(&input);
                let next = if index > 0 {
                    posts.get(index - 1).map(|post| PostLink {
                        title: post.title.clone(),
                        url: format!("/post/{}", post.url),
                    })
                } else {
                    None
                };
                let previous = posts.get(index + 1).map(|post| PostLink {
                    title: post.title.clone(),
                    url: format!("/post/{}", post.url),
                });
                layout(
                    &post.title,
                    html! {
                        h2 { (post.title) }
                        p .date {
                            "Published "
                            (format!("{}", post.published_at.format("%B %e, %Y")))
                        }
                        article { (PreEscaped(content)) }
                        (reference_link(next, "Read the next story? "))
                        (reference_link(previous, "Read the previous story? "))
                    },
                )
                .into_response()
            }
            None => (StatusCode::NOT_FOUND, "".to_string()).into_response(),
        },
        None => (StatusCode::NOT_FOUND, "".to_string()).into_response(),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("www=debug,tower_http=info"))
                .unwrap(),
        )
        .init();

    let app = Router::new()
        .nest_service("/static", ServeDir::new("dist"))
        // This is for legacy perseus build
        .route(
            "/.perseus/{*path}",
            get(async |Path(path): Path<String>| Redirect::permanent(&format!("/{}", &path))),
        )
        .route("/", get(index))
        .route("/feed.xml", get(feed))
        .route("/about", get(about))
        .route("/post/{url}", get(post))
        // Support also directory style urls
        .route("/about/", get(async || Redirect::permanent("/about")))
        .route(
            "/post/{url}/",
            get(async |Path(url): Path<String>| Redirect::permanent(&format!("/post/{}", &url))),
        )
        .route_service("/favicon.ico", ServeFile::new("dist/favicon.ico"))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        );
    let app = if std::env::var("DEV").is_ok() {
        app.layer(LiveReloadLayer::new())
    } else {
        app
    };
    let port = std::env::var("PORT").unwrap_or("3000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
