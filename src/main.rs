use axum::{
    Router,
    extract::{Path, Query, Request, State},
    handler::Handler,
    http::{HeaderName, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use maud::{DOCTYPE, Markup, PreEscaped, html};
use rss::extension::atom::{AtomExtension, Link};
use rss::{ChannelBuilder, Item};
use std::collections::HashMap;
use std::path::Path as FsPath;
use std::sync::Arc;
use tower_http::{
    compression::{
        CompressionLayer, DefaultPredicate,
        predicate::{NotForContentType, Predicate},
    },
    services::{ServeDir, ServeFile},
    trace,
    trace::TraceLayer,
};
use tower_livereload::LiveReloadLayer;
use tracing::Level;
use tracing_subscriber::EnvFilter;

use taichiwayfarer::assets;
use taichiwayfarer::data::get_extra_markup;
use taichiwayfarer::posts::{Post, PostLink, PostRepository, post_path};

const DEFAULT_SITE_URL: &str = "https://www.taichiwayfarer.com";

#[derive(Clone)]
struct AppState {
    posts: Arc<PostRepository>,
    /// Public address of the site without a trailing slash, for absolute links.
    site_url: Arc<str>,
    /// Asks search engines not to index the site, e.g. for a test copy.
    noindex: bool,
}

/// Page details for the document head, search engines and link previews.
struct PageMeta<'a> {
    site_url: &'a str,
    title: &'a str,
    description: &'a str,
    /// Canonical path of the page, or `None` for error pages.
    path: Option<&'a str>,
    image: Option<&'a str>,
    og_type: &'a str,
    /// Asks search engines not to index this page, e.g. search results.
    noindex: bool,
    /// Words in the search box of the top bar, those of the shown results.
    search_query: &'a str,
}

fn absolute_url(site_url: &str, path: &str) -> String {
    if path.starts_with("http") {
        path.to_string()
    } else {
        format!("{site_url}{path}")
    }
}

fn layout(meta: &PageMeta, content: &Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { (format!("{} ☯ Tai Chi Wayfarer", meta.title)) }
                meta name="viewport" content="width=device-width, initial-scale=1";
                // Mobile browsers tint their toolbar to match the page background (slate-900).
                meta name="theme-color" content="#0f172b";
                meta name="description" content=(meta.description);
                @if meta.noindex {
                    meta name="robots" content="noindex";
                }
                @if let Some(path) = meta.path {
                    link rel="canonical" href=(absolute_url(meta.site_url, path));
                    meta property="og:url" content=(absolute_url(meta.site_url, path));
                }
                meta property="og:site_name" content="Tai Chi Wayfarer";
                meta property="og:type" content=(meta.og_type);
                meta property="og:title" content=(meta.title);
                meta property="og:description" content=(meta.description);
                meta property="og:image" content=(absolute_url(meta.site_url, &meta.image.map_or_else(|| assets::path(assets::HEADER_PHOTO), str::to_string)));
                meta name="twitter:card" content="summary_large_image";
                link href=(assets::url(assets::STYLESHEET)) rel="stylesheet";
                // The .ico for browsers without SVG icons; sizes keeps others from preferring it.
                link rel="icon" href=(format!("/{}", assets::ICO_ICON)) sizes="32x32";
                link rel="icon" href=(assets::url(assets::SVG_ICON)) type="image/svg+xml";
                link rel="apple-touch-icon" href=(assets::url(assets::TOUCH_ICON));
                // Fetch the fonts of the title and text together with the stylesheet
                // instead of after it. The URLs must match what styles.css resolves to.
                @for font in assets::PRELOADED_FONTS {
                    link rel="preload" href=(assets::path(font)) as="font" type="font/woff2" crossorigin;
                }
                link rel="alternate" type="application/rss+xml" title="Subscribe to Posts" href="/feed.xml";
            }
            body {
                // Page regions, which screen reader users can jump between.
                header {
                    h1 { a href="/" { "Tai Chi Wayfarer" } }
                    // In reading order, so keyboard and screen reader users meet the
                    // links in the same order as they are shown.
                    div .top-bar {
                        nav {
                            ul {
                                li { a href="/" aria-current=[(meta.path == Some("/")).then_some("page")] { "Posts" } }
                                li { a href="/about" aria-current=[(meta.path == Some("/about")).then_some("page")] { "About" } }
                            }
                        }
                        // Together, so that on narrow screens they move to the next line as one.
                        div .search-and-support {
                            (search_form(meta.search_query))
                            div .support-me {
                                a .kofi-button href="https://ko-fi.com/taichiwayfarer" title="Support me on ko-fi.com" target="_blank" rel="noopener" {
                                    (static_image(assets::KOFI_ICON, None, "Support me on Ko-fi"))
                                }
                            }
                        }
                    }
                }
                main {
                    (content)
                }
            }
        }
    }
}

/// An image from the static files, with its width and height so the page
/// doesn't jump while it loads. Use the image names in `assets`, whose sizes
/// are read at startup.
fn static_image(name: &str, class: Option<&str>, alt: &str) -> Markup {
    let size = assets::image_size(name);
    html! {
        img class=[class] src=(assets::path(name)) alt=(alt)
            width=[size.map(|(width, _)| width)] height=[size.map(|(_, height)| height)];
    }
}

/// The post's publication date, readable for people and, through `datetime`,
/// for browsers and search engines.
fn date(post: &Post) -> Markup {
    html! {
        time datetime=(post.published_at.format("%Y-%m-%d")) {
            (post.published_at.format("%B %e, %Y"))
        }
    }
}

/// The top of the post list and the search results: a heading, a Lao Tzu
/// quote, and the header photo beside an introduction.
fn listing_intro(heading: &str, lao_tzu: &str, intro: &Markup) -> Markup {
    html! {
        h2 { (heading) }
        (quote(lao_tzu, "Lao Tzu"))
        (static_image(assets::HEADER_PHOTO, Some("index"), "Tai Chi Wayfarer"))
        (intro)
    }
}

/// A post's title and date, as one row of the post list and the search results.
fn post_row(post: &Post) -> Markup {
    html! {
        div {
            div { a href=(post.path()) { (post.title) } }
            div .date { (date(post)) }
        }
    }
}

fn search_form(query: &str) -> Markup {
    html! {
        form .search action="/search" method="get" role="search" {
            // Enter, or the search key of a phone keyboard, sends the form.
            input type="search" name="q" value=(query) aria-label="Search posts" placeholder="Search";
        }
    }
}

async fn search(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    let query = params.get("q").map_or("", |query| query.trim());
    // Nothing to search for: back to the list of all posts.
    if query.is_empty() {
        return Redirect::to("/").into_response();
    }
    let hits = state.posts.search(query);
    layout(
        &PageMeta {
            site_url: &state.site_url,
            title: "Search",
            description: "Search the posts of Tai Chi Wayfarer.",
            path: None,
            image: None,
            og_type: "website",
            noindex: true,
            search_query: query,
        },
        &html! {
            (listing_intro(
                "Search",
                "Without going outside, you may know the whole world.",
                &html! {
                    p .search-summary {
                        @match hits.len() {
                            0 => "No posts found",
                            1 => "1 post found",
                            count => { (count) " posts found" }
                        }
                        " for “" (query) "”."
                    }
                },
            ))
            ul .post-list.search-results {
                @for hit in &hits {
                    li {
                        (post_row(hit.post))
                        @if let Some(excerpt) = &hit.excerpt {
                            p { (excerpt.before) mark { (excerpt.matched) } (excerpt.after) }
                        }
                    }
                }
            }
        },
    )
    .into_response()
}

fn quote(text: &str, author: &str) -> Markup {
    html! {
        figure .quote {
            blockquote { (text) }
            figcaption { "- " (author) }
        }
    }
}

async fn index(State(state): State<AppState>) -> Markup {
    layout(
        &PageMeta {
            site_url: &state.site_url,
            title: "Posts",
            description: "In this blog, I share my love of Tai Chi. I hope you enjoy!",
            path: Some("/"),
            image: None,
            og_type: "website",
            noindex: false,
            search_query: "",
        },
        &html! {
            (listing_intro(
                "Posts",
                "The journey of a thousand miles begins with one step.",
                &html! {
                    p { "In this blog, I share my love of Tai Chi. I hope you enjoy!" }
                    p {
                        "If you'd like to hear about upcoming camps and workshops or when I publish new posts, "
                        a href="mailto:taichiwayfarer@gmail.com?subject=Subscribe&body=Please tell me something about yourself. Who is your teacher, or what kind of content are you interested in?" { "send me an email" }
                        ", and I will share the flyers and my updates with you."
                    }
                },
            ))
            ul .post-list {
                @for post in &state.posts.post_list {
                    li { (post_row(post)) }
                }
            }
            p .reference {
                a href="/feed.xml" rel="external" {
                    (static_image(assets::RSS_ICON, Some("rss"), "Subscribe to Posts"))
                }
            }
        },
    )
}

async fn feed(State(state): State<AppState>) -> impl IntoResponse {
    let items: Vec<Item> = state
        .posts
        .post_list
        .iter()
        .map(|post| {
            let mut item = post.to_item(&state.site_url);
            item.description = state
                .posts
                .post_map
                .get(&post.url)
                .map(|page| page.description.clone());
            item
        })
        .collect();
    let pub_date = items.first().and_then(|item| item.pub_date.clone());
    let mut channel = ChannelBuilder::default()
        .title("Tai Chi Wayfarer")
        .link(state.site_url.to_string())
        .description("RSS feed of Tai Chi Wayfarer posts.")
        .pub_date(pub_date)
        .items(items)
        .build();
    // The feed's own address, which feed validators ask for.
    let mut self_link = Link::default();
    self_link.set_href(format!("{}/feed.xml", state.site_url));
    self_link.set_rel("self");
    self_link.set_mime_type(Some("application/rss+xml".to_string()));
    channel.set_atom_ext(AtomExtension {
        links: vec![self_link],
    });
    (
        [(header::CONTENT_TYPE, "text/xml; charset=utf-8")],
        channel.to_string(),
    )
}

async fn sitemap(State(state): State<AppState>) -> impl IntoResponse {
    let xml = html! {
        (PreEscaped(r#"<?xml version="1.0" encoding="UTF-8"?>"#))
        urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" {
            @for path in ["/", "/about"] {
                url { loc { (state.site_url) (path) } }
            }
            @for post in &state.posts.post_list {
                url {
                    loc { (state.site_url) (post.path()) }
                    lastmod { (post.published_at.format("%Y-%m-%d")) }
                }
            }
        }
    };
    (
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml.into_string(),
    )
}

async fn robots(State(state): State<AppState>) -> impl IntoResponse {
    let body = if state.noindex {
        "User-agent: *\nDisallow: /\n".to_string()
    } else {
        format!(
            "User-agent: *\nDisallow:\n\nSitemap: {}/sitemap.xml\n",
            state.site_url
        )
    };
    ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body)
}

/// Set (even empty, like `DEV=` in the Justfile) and not "0" or "false".
fn env_flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| !matches!(value.as_str(), "0" | "false"))
}

async fn add_noindex(mut response: Response) -> Response {
    response.headers_mut().insert(
        HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex"),
    );
    response
}

/// Sets `Cache-Control` for static files. Versioned URLs (`?v=<hash>`, see
/// `assets::url`) change with their content, so they are cached for a year;
/// other files for a day. Errors are not cached, and nothing is in dev mode.
async fn cache_static(State(dev): State<bool>, request: Request, next: Next) -> Response {
    let versioned = request
        .uri()
        .path()
        .strip_prefix("/static/")
        .zip(
            request
                .uri()
                .query()
                .and_then(|query| query.strip_prefix("v=")),
        )
        .is_some_and(|(name, version)| assets::is_current(name, version));
    let mut response = next.run(request).await;
    if response.status().is_success() {
        let value = if dev {
            "no-cache"
        } else if versioned {
            "public, max-age=31536000, immutable"
        } else {
            "public, max-age=86400"
        };
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    }
    response
}

async fn about(State(state): State<AppState>) -> Markup {
    let profile_photo = assets::path(assets::PROFILE_PHOTO);
    layout(
        &PageMeta {
            site_url: &state.site_url,
            title: "About",
            description: "Hi, my name is Timo. I've been practicing Tai Chi Chuan since 2002.",
            path: Some("/about"),
            image: Some(&profile_photo),
            og_type: "profile",
            noindex: false,
            search_query: "",
        },
        &html! {
            article {
                h2 { "About" }
                (quote("Be still like a mountain and flow like a great river.", "Lao Tzu"))
                (static_image(assets::PROFILE_PHOTO, Some("index"), "Profile picture"))
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
                a href=(x.path()) { (x.title) }
            }
        },
    }
}

async fn post(Path(url): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    match state.posts.post_map.get(&url) {
        Some(page) => {
            let extra_markup = get_extra_markup(&page.post.url);
            let path = page.post.path();
            layout(
                &PageMeta {
                    site_url: &state.site_url,
                    title: &page.post.title,
                    description: &page.description,
                    path: Some(&path),
                    image: page.image.as_deref(),
                    og_type: "article",
                    noindex: false,
                    search_query: "",
                },
                &html! {
                    h2 { (page.post.title) }
                    p .date { "Published " (date(&page.post)) }
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
        None => not_found(&state.site_url),
    }
}

async fn post_redirect(
    Path(url): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if state.posts.post_map.contains_key(&url) {
        Redirect::permanent(&post_path(&url)).into_response()
    } else {
        not_found(&state.site_url)
    }
}

fn app(state: AppState, static_dir: &FsPath, dev: bool) -> Router {
    let noindex = state.noindex;
    let static_files = Router::new()
        // Missing files get the same not found page as missing pages.
        .nest_service(
            "/static",
            ServeDir::new(static_dir).not_found_service(handler_404.with_state(state.clone())),
        )
        .route_service(
            &format!("/{}", assets::ICO_ICON),
            ServeFile::new(static_dir.join(assets::ICO_ICON)),
        )
        .layer(middleware::from_fn_with_state(dev, cache_static));

    let app = Router::new()
        .merge(static_files)
        .route("/", get(index))
        .route("/feed.xml", get(feed))
        .route("/sitemap.xml", get(sitemap))
        .route("/robots.txt", get(robots))
        .route("/about", get(about))
        .route("/search", get(search))
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
    let app = if noindex {
        app.layer(middleware::map_response(add_noindex))
    } else {
        app
    };
    let app = if dev {
        app.layer(LiveReloadLayer::new())
    } else {
        app
    };
    // Outermost, so that live reload can still inject into uncompressed HTML.
    // Images are already skipped by the default predicate, and so are fonts here.
    app.layer(
        CompressionLayer::new()
            .compress_when(DefaultPredicate::new().and(NotForContentType::const_new("font/woff2"))),
    )
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
    let static_dir = FsPath::new(&static_dir);
    // A wrong STATIC_DIR would otherwise serve pages without styles or images.
    let missing = assets::missing(static_dir);
    if !missing.is_empty() {
        tracing::error!(static_dir = %static_dir.display(), ?missing, "Static files not found, check STATIC_DIR");
        return Err(format!(
            "static files not found in {}: {}",
            static_dir.display(),
            missing.join(", ")
        )
        .into());
    }
    assets::init(static_dir);
    let posts = PostRepository::new(FsPath::new(&post_dir), static_dir).map_err(|error| {
        tracing::error!(%error, "Failed to load posts");
        error
    })?;
    let site_url = std::env::var("SITE_URL").unwrap_or(DEFAULT_SITE_URL.to_string());
    let state = AppState {
        posts: Arc::new(posts),
        site_url: site_url.trim_end_matches('/').into(),
        noindex: env_flag("NOINDEX"),
    };
    tracing::info!(site_url = %state.site_url, noindex = state.noindex, "Site settings");

    let dev = env_flag("DEV");
    let app = app(state, static_dir, dev);
    // Behind a reverse proxy, set HOST=127.0.0.1 to accept local connections only.
    let host = std::env::var("HOST").unwrap_or("0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or("3000".to_string());
    let bind_addr = if host.contains(':') {
        format!("[{host}]:{port}") // IPv6
    } else {
        format!("{host}:{port}")
    };
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
    let serve = axum::serve(listener, app.into_make_service());
    let result = if dev {
        // Live reload keeps an event stream open for each browser tab, which a
        // graceful shutdown would wait for, so dev restarts stop at once.
        tokio::select! {
            result = serve.into_future() => result,
            () = shutdown_signal() => Ok(()),
        }
    } else {
        // Finish requests in progress, e.g. when systemd stops the service.
        serve.with_graceful_shutdown(shutdown_signal()).await
    };
    if let Err(error) = result {
        tracing::error!(address = %local_addr, %error, "HTTP server failed");
        return Err(error.into());
    }
    tracing::info!(address = %local_addr, "HTTP server stopped");
    Ok(())
}

/// Completes on Ctrl-C or, on Unix, SIGTERM (sent e.g. by systemd and watchexec).
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "Failed to listen for Ctrl-C");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::error!(%error, "Failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    tracing::info!("Shutdown signal received");
}

async fn handler_404(State(state): State<AppState>) -> Response {
    not_found(&state.site_url)
}

fn not_found(site_url: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        layout(
            &PageMeta {
                site_url,
                title: "Not Found",
                description: "The page you are looking for does not exist.",
                path: None,
                image: None,
                og_type: "website",
                noindex: false,
                search_query: "",
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
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use axum::http::HeaderMap;
    use tower::ServiceExt;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn test_app(site_url: &str, noindex: bool) -> TestResult<Router> {
        let root = FsPath::new(env!("CARGO_MANIFEST_DIR"));
        assets::init(&root.join("dist"));
        let state = AppState {
            posts: Arc::new(PostRepository::new(
                &root.join("posts"),
                &root.join("dist"),
            )?),
            site_url: site_url.into(),
            noindex,
        };
        Ok(app(state, &root.join("dist"), false))
    }

    async fn fetch(app: &Router, uri: &str) -> TestResult<(HeaderMap, String)> {
        let request = Request::builder().uri(uri).body(Body::empty())?;
        let response = app.clone().oneshot(request).await?;
        let headers = response.headers().clone();
        let body = to_bytes(response.into_body(), usize::MAX).await?;
        Ok((headers, String::from_utf8(body.to_vec())?))
    }

    #[tokio::test]
    async fn links_use_site_url() -> TestResult {
        let app = test_app("https://alt.example.com", false)?;

        let (headers, page) = fetch(&app, "/post/the-slow-set").await?;
        assert!(
            page.contains(r#"rel="canonical" href="https://alt.example.com/post/the-slow-set""#)
        );
        assert!(!headers.contains_key("x-robots-tag"));

        let (_, feed) = fetch(&app, "/feed.xml").await?;
        assert!(feed.contains("<link>https://alt.example.com/post/the-slow-set</link>"));

        let (_, sitemap) = fetch(&app, "/sitemap.xml").await?;
        assert!(sitemap.contains("<loc>https://alt.example.com/about</loc>"));

        let (_, robots) = fetch(&app, "/robots.txt").await?;
        assert!(robots.contains("Sitemap: https://alt.example.com/sitemap.xml"));
        assert!(robots.contains("Disallow:\n"));
        Ok(())
    }

    #[tokio::test]
    async fn missing_static_file_gets_not_found_page() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let request = Request::builder()
            .uri("/static/no-such-file.css")
            .body(Body::empty())?;
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert!(!response.headers().contains_key(header::CACHE_CONTROL));
        let body = to_bytes(response.into_body(), usize::MAX).await?;
        assert!(String::from_utf8(body.to_vec())?.contains("<h2>Not Found</h2>"));
        Ok(())
    }

    #[tokio::test]
    async fn every_post_link_resolves() -> TestResult {
        // post_path formats the links and the router's patterns serve them, so
        // following every link on the index checks that the two agree.
        let app = test_app("https://www.example.com", false)?;
        let (_, index) = fetch(&app, "/").await?;
        let links: Vec<&str> = index
            .split(r#"href=""#)
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .filter(|href| href.starts_with("/post/"))
            .collect();
        assert!(links.len() > 30, "{links:?}");
        for link in links {
            let request = Request::builder().uri(link).body(Body::empty())?;
            let response = app.clone().oneshot(request).await?;
            assert_eq!(response.status(), StatusCode::OK, "{link}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn feed_links_to_itself() -> TestResult {
        let app = test_app("https://alt.example.com", false)?;
        let (_, feed) = fetch(&app, "/feed.xml").await?;
        assert!(
            feed.contains(r#"xmlns:atom="http://www.w3.org/2005/Atom""#),
            "{feed}"
        );
        assert!(feed.contains(
            r#"<atom:link href="https://alt.example.com/feed.xml" rel="self" type="application/rss+xml"/>"#
        ), "{feed}");
        Ok(())
    }

    #[tokio::test]
    async fn dates_are_machine_readable() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let (_, index) = fetch(&app, "/").await?;
        assert!(index.contains(r#"<time datetime="2026-10-24">October 24, 2026</time>"#));
        let (_, post) = fetch(&app, "/post/the-slow-set").await?;
        assert!(post.contains(r#"Published <time datetime="2026-08-18">August 18, 2026</time>"#));
        Ok(())
    }

    #[tokio::test]
    async fn pages_have_header_and_main_regions() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        for uri in ["/", "/about", "/post/the-slow-set", "/missing"] {
            let (_, page) = fetch(&app, uri).await?;
            let header = page
                .find("<header><h1>")
                .ok_or(format!("no header on {uri}"))?;
            let main = page.find("<main>").ok_or(format!("no main on {uri}"))?;
            assert!(header < main && page.contains("</main></body>"), "{uri}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn search_page_lists_matches() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let (_, page) = fetch(&app, "/search?q=man+quan").await?;
        assert!(page.contains(r#"<meta name="robots" content="noindex">"#));
        assert!(page.contains(r#"value="man quan""#));
        // The same rows of title and date as the post list.
        assert!(
            page.contains(r#"<ul class="post-list search-results"><li><div><div><a href="/post/the-slow-set">The Slow Set</a></div><div class="date"><time datetime="2026-08-18">"#),
            "{page}"
        );
        assert!(page.contains("<mark>màn quán</mark>"), "{page}");

        let (_, none) = fetch(&app, "/search?q=qwertyuiop").await?;
        assert!(none.contains("No posts found for “qwertyuiop”."));

        // The query is shown as text, never as markup.
        let (_, escaped) = fetch(&app, "/search?q=%3Cscript%3E").await?;
        assert!(!escaped.contains("<script>") && escaped.contains("&lt;script&gt;"));

        // An empty search goes back to the list of all posts.
        for uri in ["/search", "/search?q=", "/search?q=+++"] {
            let request = Request::builder().uri(uri).body(Body::empty())?;
            let response = app.clone().oneshot(request).await?;
            assert_eq!(response.status(), StatusCode::SEE_OTHER, "{uri}");
            assert_eq!(
                response
                    .headers()
                    .get(header::LOCATION)
                    .map(HeaderValue::as_bytes),
                Some(&b"/"[..]),
                "{uri}"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn search_page_looks_like_post_list() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let (_, page) = fetch(&app, "/search?q=sword").await?;
        assert!(page.contains("<main><h2>Search</h2><figure class=\"quote\">"));
        assert!(page.contains("Without going outside, you may know the whole world."));
        assert!(page.contains(r#"<img class="index" src="/static/img/2023-to-the-states.jpg""#));
        Ok(())
    }

    #[tokio::test]
    async fn every_page_has_search_in_top_bar() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let form = r#"</nav><div class="search-and-support"><form class="search" action="/search" method="get" role="search">"#;
        for uri in [
            "/",
            "/about",
            "/post/the-slow-set",
            "/missing",
            "/search?q=sword",
        ] {
            let (_, page) = fetch(&app, uri).await?;
            assert!(page.contains(form), "{uri}");
            // Only once: the results page has no second search box.
            assert_eq!(page.matches(r#"name="q""#).count(), 1, "{uri}");
        }
        let (_, index) = fetch(&app, "/").await?;
        assert!(!index.contains(r#"name="robots""#));
        Ok(())
    }

    #[tokio::test]
    async fn icons_are_linked() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let (_, page) = fetch(&app, "/").await?;
        assert!(page.contains(r#"<link rel="icon" href="/favicon.ico" sizes="32x32">"#));
        for (rel, name) in [
            ("icon", "favicon.svg"),
            ("apple-touch-icon", "apple-touch-icon.png"),
        ] {
            let link = format!(r#"<link rel="{rel}" href="/static/{name}?v="#);
            assert!(page.contains(&link), "{link}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn layout_images_have_sizes() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let dist = FsPath::new(env!("CARGO_MANIFEST_DIR")).join("dist");
        for (uri, name) in [
            ("/", "img/2023-to-the-states.jpg"),
            ("/", "img/rss.png"),
            ("/about", "img/profile.jpg"),
            ("/about", "img/kofi-cup.png"),
        ] {
            let size = imagesize::size(dist.join(name))?;
            let (_, page) = fetch(&app, uri).await?;
            let tag = page
                .split("<img ")
                .find(|tag| tag.contains(&format!(r#"src="/static/{name}""#)))
                .ok_or(format!("no {name} on {uri}"))?;
            let expected = format!(r#"width="{}" height="{}""#, size.width, size.height);
            assert!(tag.contains(&expected), "{name}: {tag}");
        }
        Ok(())
    }

    #[tokio::test]
    async fn story_links_are_absolute() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let (_, page) = fetch(&app, "/post/the-slow-set").await?;
        assert!(page.contains(r#"Read the next story? <a href="/post/learn-by-watching">"#));
        assert!(page.contains(r#"Read the previous story? <a href="/post/june-2026">"#));
        Ok(())
    }

    #[tokio::test]
    async fn nav_marks_current_page() -> TestResult {
        let app = test_app("https://www.example.com", false)?;
        let (_, about) = fetch(&app, "/about").await?;
        assert!(about.contains(r#"<a href="/about" aria-current="page">About</a>"#));
        assert!(about.contains(r#"<a href="/">Posts</a>"#));
        // Keyboard and screen reader order must match the visual order.
        let posts = about.find(">Posts</a>").ok_or("no Posts link")?;
        let about_link = about.find(">About</a>").ok_or("no About link")?;
        let search = about.find(r#"name="q""#).ok_or("no search box")?;
        let kofi = about.find("ko-fi.com").ok_or("no Ko-fi link")?;
        assert!(posts < about_link && about_link < search && search < kofi);
        let (_, post) = fetch(&app, "/post/the-slow-set").await?;
        assert!(!post.contains("aria-current"));
        Ok(())
    }

    #[tokio::test]
    async fn noindex_blocks_search_engines() -> TestResult {
        let app = test_app("https://alt.example.com", true)?;

        let (_, robots) = fetch(&app, "/robots.txt").await?;
        assert_eq!(robots, "User-agent: *\nDisallow: /\n");

        for uri in [
            "/",
            "/post/the-slow-set",
            "/static/embed-video.js",
            "/missing",
        ] {
            let (headers, _) = fetch(&app, uri).await?;
            assert_eq!(
                headers.get("x-robots-tag").map(HeaderValue::as_bytes),
                Some(&b"noindex"[..]),
                "{uri}"
            );
        }
        Ok(())
    }
}
