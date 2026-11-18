use axum::{
    Router,
    extract::{Form, Path, Query, Request, State},
    handler::Handler,
    http::{HeaderName, HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::get,
};
use chrono::{DateTime, Utc};
use lettre::message::Mailbox;
use maud::{DOCTYPE, Markup, PreEscaped, html};
use rss::extension::atom::{AtomExtension, Link};
use rss::{ChannelBuilder, Item};
use std::collections::HashMap;
use std::path::Path as FsPath;
use std::sync::Arc;
use tokio::sync::Notify;
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
use taichiwayfarer::db;
use taichiwayfarer::mail::{self, Encryption, Mailer};
use taichiwayfarer::newsletter::Newsletter;
use taichiwayfarer::posts::{Post, PostImage, PostLink, PostRepository};
use taichiwayfarer::subscribers::{NOTE_LENGTH, Signup, Store, parse_email};

const DEFAULT_SITE_URL: &str = "https://www.taichiwayfarer.com";

#[derive(Clone)]
struct AppState {
    posts: Arc<PostRepository>,
    /// Public address of the site without a trailing slash, for absolute links.
    site_url: Arc<str>,
    /// Asks search engines not to index the site, e.g. for a test copy.
    noindex: bool,
    /// The current time, which decides which posts are published. A function,
    /// so that tests can pick the time.
    now: fn() -> DateTime<Utc>,
    /// The subscribers to new posts by email.
    subscribers: Store,
    /// Wakes the newsletter to send a confirmation email at once.
    wake: Arc<Notify>,
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
                        // Named, as posts also have a "More stories" navigation.
                        nav aria-label="Main" {
                            ul {
                                li { (nav_link("/", "Posts", meta.path)) }
                                li { (nav_link("/about", "About", meta.path)) }
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

/// A link of the top bar, marked when it is the page the reader is on. Only
/// then: from a post, Posts leads to the list of posts, so it stays a link.
fn nav_link(href: &str, label: &str, path: Option<&str>) -> Markup {
    html! {
        a href=(href) aria-current=[(path == Some(href)).then_some("page")] { (label) }
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
/// quote, and a photo beside an introduction.
fn listing_intro(heading: &str, lao_tzu: &str, photo: &Markup, intro: &Markup) -> Markup {
    html! {
        h2 { (heading) }
        (quote(lao_tzu, "Lao Tzu"))
        (photo)
        (intro)
    }
}

/// The photo of the post list, also used where nothing better is at hand.
/// Decoration only, so screen readers skip it.
fn header_photo() -> Markup {
    static_image(assets::HEADER_PHOTO, Some("index"), "")
}

/// A post's image beside the introduction of the search results.
fn post_photo(image: &PostImage) -> Markup {
    html! {
        img .index src=(image.src) alt=(image.alt)
            width=[image.size.map(|(width, _)| width)] height=[image.size.map(|(_, height)| height)];
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
    let hits = state.posts.search(query, (state.now)());
    // The first photo of the first result that has one, as a glimpse of what was found.
    let photo = hits
        .iter()
        .find_map(|hit| state.posts.post_map.get(&hit.post.url)?.image.as_ref())
        .map_or_else(header_photo, post_photo);
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
                &photo,
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
                &header_photo(),
                &html! {
                    p { "In this blog, I share my love of Tai Chi. I hope you enjoy!" }
                    p {
                        "If you'd like to hear when I publish new posts, "
                        a href="/subscribe" { "subscribe by email" }
                        ", and I'll send each new one to you."
                    }
                },
            ))
            ul .post-list {
                @for post in state.posts.published((state.now)()) {
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
        .published((state.now)())
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
            @for post in state.posts.published((state.now)()) {
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

/// The time that decides which posts are published. In dev mode it is far in
/// the future, so scheduled posts show up while they are being written.
fn clock(dev: bool) -> fn() -> DateTime<Utc> {
    if dev {
        || DateTime::<Utc>::MAX_UTC
    } else {
        Utc::now
    }
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

/// Signing up for new posts by email, with an optional note about oneself,
/// on `/subscribe` only, as is the link to the privacy notice. The values and
/// the error are those of a sign-up that didn't go through.
fn subscribe_form(email: &str, note: &str, error: Option<&str>) -> Markup {
    html! {
        // In a box of its own under the page's introduction and photo.
        form .subscribe.signup method="post" action="/subscribe" {
            fieldset {
                legend { "New posts by email" }
                @if let Some(error) = error {
                    p .form-error role="alert" { (error) }
                }
                label for="subscribe-email" { "Your email" }
                input #subscribe-email type="email" name="email" value=(email) required autocomplete="email";
                label for="subscribe-note" { "About you (optional)" }
                textarea #subscribe-note name="note" rows="3" maxlength=(NOTE_LENGTH)
                    placeholder="Who is your teacher, or what would you like to read about?" { (note) }
                // Hidden from people; bots that fill in every field fill it in too.
                div .trap aria-hidden="true" {
                    label for="subscribe-website" { "Leave this empty" }
                    input #subscribe-website type="text" name="website" tabindex="-1" autocomplete="off";
                }
                button type="submit" { "Subscribe" }
                p .fine-print {
                    "I use your email only for my posts, and every post email has a link to unsubscribe. See "
                    a href="/privacy" { "privacy" }
                    "."
                }
            }
        }
    }
}

/// A Lao Tzu quote and a photo beside the text, as the list of posts opens,
/// for the pages of confirming and unsubscribing.
struct Opening {
    quote: &'static str,
    photo: &'static str,
}

/// Waiting for the confirmation email.
const INBOX: Opening = Opening {
    quote: "Do you have the patience to wait till your mud settles and the water is clear?",
    photo: assets::INBOX_PHOTO,
};
/// Confirming, and confirmed.
const SUBSCRIBED: Opening = Opening {
    quote: "The sage does not hoard. The more he gives to others, the more he has.",
    photo: assets::SUBSCRIBED_PHOTO,
};
/// Unsubscribing, and unsubscribed.
const UNSUBSCRIBE: Opening = Opening {
    quote: "Do your work, then step back. The only path to serenity.",
    photo: assets::UNSUBSCRIBE_PHOTO,
};

/// A short page about subscribing, which search engines leave out, opened
/// with a quote and a photo, or, for errors, without.
fn subscription_page(
    state: &AppState,
    status: StatusCode,
    title: &str,
    opening: Option<&Opening>,
    body: &Markup,
) -> Response {
    (
        status,
        layout(
            &PageMeta {
                site_url: &state.site_url,
                title,
                description: "Get the new posts of Tai Chi Wayfarer by email.",
                path: None,
                image: None,
                og_type: "website",
                noindex: true,
                search_query: "",
            },
            &html! {
                // Left-aligned: justifying stretches the words of these few
                // lines, e.g. before a long email address.
                div .short-page {
                    @if let Some(opening) = opening {
                        // The photo is decoration, as the photo of the post list.
                        (listing_intro(title, opening.quote, &static_image(opening.photo, Some("index"), ""), body))
                    } @else {
                        h2 { (title) }
                        (body)
                    }
                }
            },
        ),
    )
        .into_response()
}

fn server_error(state: &AppState, error: &sqlx::Error) -> Response {
    tracing::error!(%error, "Subscribers database failed");
    subscription_page(
        state,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Something Went Wrong",
        None,
        &html! { p { "Sorry, that didn't work. Please try again later." } },
    )
}

/// The page of signing up, linked from the front page.
async fn subscribe_page(State(state): State<AppState>) -> Response {
    signup_page(&state, StatusCode::OK, &subscribe_form("", "", None))
}

/// The page of signing up, with a Lao Tzu quote and a photo as the list of
/// posts, around `form`.
fn signup_page(state: &AppState, status: StatusCode, form: &Markup) -> Response {
    (
        status,
        layout(
            &PageMeta {
                site_url: &state.site_url,
                title: "Subscribe",
                description: "Get the new posts of Tai Chi Wayfarer by email.",
                path: None,
                image: None,
                og_type: "website",
                noindex: true,
                search_query: "",
            },
            &html! {
                (listing_intro(
                    "Subscribe",
                    "A good traveler has no fixed plans and is not intent upon arriving.",
                    // Decoration, as the photo of the post list.
                    &static_image(assets::SUBSCRIBE_PHOTO, Some("index"), ""),
                    &html! {
                        p {
                            "Get each new post of Tai Chi Wayfarer by email. "
                            "Tell me something about yourself too, if you like."
                        }
                    },
                ))
                (form)
            },
        ),
    )
        .into_response()
}

async fn subscribe(
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> Response {
    let field = |name: &str| form.get(name).map_or("", String::as_str);
    let check_inbox = |email: &str| {
        subscription_page(
            &state,
            StatusCode::OK,
            "Check Your Inbox",
            Some(&INBOX),
            &html! {
                p { "Thank you! To confirm your subscription, click the link in the email I sent to " strong { (email) } "." }
                p { "If it doesn't arrive in a few minutes, please look in your spam folder." }
            },
        )
    };
    // A bot: thank it as anyone, but don't email anybody.
    if !field("website").is_empty() {
        tracing::info!("Ignored a sign-up with the hidden field filled in");
        return check_inbox(field("email").trim());
    }
    let Some(email) = parse_email(field("email")) else {
        return signup_page(
            &state,
            StatusCode::UNPROCESSABLE_ENTITY,
            &subscribe_form(
                field("email"),
                field("note"),
                Some("Please check your email address, it doesn't look like one."),
            ),
        );
    };
    match state
        .subscribers
        .subscribe(&email, field("note"), Utc::now())
        .await
    {
        Ok(Signup::TooMany) => subscription_page(
            &state,
            StatusCode::SERVICE_UNAVAILABLE,
            "Please Try Again Later",
            None,
            &html! { p { "Many people are signing up just now. Please try again in an hour." } },
        ),
        Ok(signup) => {
            if signup == Signup::ConfirmationQueued {
                tracing::info!("New sign-up, sending the confirmation");
                state.wake.notify_one();
            }
            check_inbox(&email)
        }
        Err(error) => server_error(&state, &error),
    }
}

fn token(params: &HashMap<String, String>) -> &str {
    params.get("token").map_or("", String::as_str)
}

fn invalid_link(state: &AppState) -> Response {
    subscription_page(
        state,
        StatusCode::NOT_FOUND,
        "Link Not Valid",
        None,
        &html! {
            p {
                "This link isn't valid anymore. Sign-ups not confirmed within a week are removed, but you can "
                a href="/subscribe" { "subscribe again" }
                "."
            }
        },
    )
}

fn subscribed(state: &AppState, email: &str) -> Response {
    subscription_page(
        state,
        StatusCode::OK,
        "You're Subscribed",
        Some(&SUBSCRIBED),
        &html! {
            p { "Thank you! From now on, I'll send each new post to " strong { (email) } "." }
            p .reference { "Meanwhile, browse " a href="/" { "all posts" } "." }
        },
    )
}

/// The link of the confirmation email: a button, as link checkers of mail
/// services open links but don't press buttons.
async fn confirm_page(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    match state.subscribers.by_token(token(&params)).await {
        Ok(Some(subscriber)) if subscriber.confirmed_at.is_some() => {
            subscribed(&state, &subscriber.email)
        }
        Ok(Some(subscriber)) => subscription_page(
            &state,
            StatusCode::OK,
            "Confirm Your Subscription",
            Some(&SUBSCRIBED),
            &html! {
                p { "Get the new posts of Tai Chi Wayfarer at " strong { (subscriber.email) } "?" }
                form .subscribe method="post" action={ "/confirm?token=" (subscriber.token) } {
                    button type="submit" { "Confirm" }
                }
            },
        ),
        Ok(None) => invalid_link(&state),
        Err(error) => server_error(&state, &error),
    }
}

async fn confirm(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    match state.subscribers.confirm(token(&params), Utc::now()).await {
        Ok(Some(subscriber)) => {
            state.wake.notify_one();
            subscribed(&state, &subscriber.email)
        }
        Ok(None) => invalid_link(&state),
        Err(error) => server_error(&state, &error),
    }
}

/// The link at the end of each email: a button, as for confirming.
async fn unsubscribe_page(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    match state.subscribers.by_token(token(&params)).await {
        Ok(Some(subscriber)) => subscription_page(
            &state,
            StatusCode::OK,
            "Unsubscribe",
            Some(&UNSUBSCRIBE),
            &html! {
                p { "Stop getting new posts at " strong { (subscriber.email) } "?" }
                form .subscribe method="post" action={ "/unsubscribe?token=" (subscriber.token) } {
                    button type="submit" { "Unsubscribe" }
                }
            },
        ),
        Ok(None) => unsubscribed(&state),
        Err(error) => server_error(&state, &error),
    }
}

/// The button, and mail apps' one-click unsubscribe (RFC 8058), which posts
/// `List-Unsubscribe=One-Click` to the link.
async fn unsubscribe(
    Query(params): Query<HashMap<String, String>>,
    State(state): State<AppState>,
) -> Response {
    match state.subscribers.unsubscribe(token(&params)).await {
        Ok(removed) => {
            if removed {
                tracing::info!("Someone unsubscribed");
            }
            unsubscribed(&state)
        }
        Err(error) => server_error(&state, &error),
    }
}

fn unsubscribed(state: &AppState) -> Response {
    subscription_page(
        state,
        StatusCode::OK,
        "You're Unsubscribed",
        Some(&UNSUBSCRIBE),
        &html! {
            p { "You won't get more posts by email, and your address is deleted from the list. You're welcome back any time." }
        },
    )
}

async fn privacy(State(state): State<AppState>) -> Markup {
    layout(
        &PageMeta {
            site_url: &state.site_url,
            title: "Privacy",
            description: "How Tai Chi Wayfarer handles the email addresses of its subscribers.",
            path: Some("/privacy"),
            image: None,
            og_type: "website",
            noindex: false,
            search_query: "",
        },
        &html! {
            article {
                h2 { "Privacy" }
                (quote("Those who know do not speak. Those who speak do not know.", "Lao Tzu"))
                // Decoration, as the photo of the post list.
                (static_image(assets::PRIVACY_PHOTO, Some("index"), ""))
                p {
                    "When you subscribe to new posts, I, Timo, the writer of Tai Chi Wayfarer, keep your email address, what you tell about yourself, and when you subscribed and confirmed. "
                    "You can reach me at taichiwayfarer@gmail.com."
                }
                p {
                    "I use them to send you new posts, because you asked for them. "
                    "I don't share them with anyone, and I don't track whether you open the emails."
                }
                p {
                    "The list is kept on the server of this blog. The emails are sent through Gmail, so Google handles them too, and Gmail keeps a copy of each sent email."
                }
                p {
                    "You can unsubscribe with the link in any post email, and your address is deleted from the list at once. "
                    "A sign-up that isn't confirmed within a week is deleted too. "
                    "You can also ask me what I have about you, or to correct or delete it, and complain to the data protection authority of your country."
                }
            }
        },
    )
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
                (static_image(assets::PROFILE_PHOTO, Some("index"), "Timo practicing Tai Chi in a garden"))
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

async fn post_page(Path(url): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    let now = (state.now)();
    match state.posts.published_page(&url, now) {
        Some(page) => {
            // Older posts are always published before this one, newer ones may not be.
            let next_link = page
                .next_link
                .as_ref()
                .filter(|next| state.posts.published_page(&next.url, now).is_some());
            let extra_markup = get_extra_markup(&page.post.url);
            let path = page.post.path();
            layout(
                &PageMeta {
                    site_url: &state.site_url,
                    title: &page.post.title,
                    description: &page.description,
                    path: Some(&path),
                    image: page.image.as_ref().map(|image| image.src.as_str()),
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
                    // At the end of the story, where readers look for what's next.
                    nav .more-stories aria-label="More stories" {
                        (reference_link(next_link, "Read the next story? "))
                        (reference_link(page.previous_link.as_ref(), "Read the previous story? "))
                        p .reference { "Or browse " a href="/" { "all posts" } "." }
                    }
                },
            )
            .into_response()
        }
        None => not_found(&state.site_url),
    }
}

/// Redirects directory style URLs, such as `/about/` and `/post/x/` of the
/// static site before, permanently to the page without the slash, keeping
/// the query. Any page gets this, and the page decides whether it exists:
/// e.g. a scheduled post's is not found either way.
async fn trim_trailing_slash(request: Request, next: Next) -> Response {
    let path = request.uri().path();
    let trimmed = path.trim_end_matches('/');
    // Not `//host` or `/\host`, which browsers would take as another site.
    let local = trimmed.len() > 1 && !trimmed[1..].starts_with(['/', '\\']);
    let read = matches!(*request.method(), Method::GET | Method::HEAD);
    if trimmed.len() < path.len() && local && read {
        let location = match request.uri().query() {
            Some(query) => format!("{trimmed}?{query}"),
            None => trimmed.to_string(),
        };
        return Redirect::permanent(&location).into_response();
    }
    next.run(request).await
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
        .route("/subscribe", get(subscribe_page).post(subscribe))
        .route("/confirm", get(confirm_page).post(confirm))
        .route("/unsubscribe", get(unsubscribe_page).post(unsubscribe))
        .route("/privacy", get(privacy))
        .route("/post/{url}", get(post_page))
        .fallback(handler_404)
        // Also for paths without a route, as /about/: the fallback has it too.
        .layer(middleware::from_fn(trim_trailing_slash))
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

    let database_url = std::env::var("DATABASE_URL").unwrap_or(db::DEFAULT_URL.to_string());
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|command| command == "subscribers") {
        if let Err(error) = subscribers_command(&args[1..], &database_url).await {
            use std::io::Write;
            let _ = writeln!(std::io::stderr(), "{error}");
            std::process::exit(1);
        }
        return Ok(());
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
    let dev = env_flag("DEV");
    let pool = db::open(&database_url).await.map_err(|error| {
        tracing::error!(%error, url = database_url, "Failed to open the database, check DATABASE_URL");
        error
    })?;
    let state = AppState {
        posts: Arc::new(posts),
        site_url: site_url.trim_end_matches('/').into(),
        noindex: env_flag("NOINDEX"),
        now: clock(dev),
        subscribers: Store::new(pool),
        wake: Arc::new(Notify::new()),
    };
    tracing::info!(site_url = %state.site_url, noindex = state.noindex, dev, "Site settings");
    if dev {
        tracing::info!("Dev mode: showing scheduled posts as published");
    }
    let newsletter = newsletter(&state).map_err(|error| {
        tracing::error!(%error, "Failed to set up sending emails");
        error
    })?;
    tokio::spawn(newsletter.run(state.wake.clone()));

    serve(app(state, static_dir, dev), dev).await
}

/// Serves the site until Ctrl-C or SIGTERM.
async fn serve(app: Router, dev: bool) -> Result<(), Box<dyn std::error::Error>> {
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

/// The sender of the emails, as configured by `MAIL_FROM`, `NOTIFY_EMAIL`
/// and the `SMTP_` variables; without `SMTP_HOST` the emails are only logged.
fn newsletter(state: &AppState) -> Result<Newsletter, String> {
    let setting = |name: &str| {
        std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
    };
    let from: Mailbox = setting("MAIL_FROM")
        .unwrap_or(mail::DEFAULT_FROM.to_string())
        .parse()
        .map_err(|error| format!("MAIL_FROM isn't an address: {error}"))?;
    let owner: Mailbox = match setting("NOTIFY_EMAIL") {
        Some(owner) => owner
            .parse()
            .map_err(|error| format!("NOTIFY_EMAIL isn't an address: {error}"))?,
        None => Mailbox::new(None, from.email.clone()),
    };
    let mailer = match setting("SMTP_HOST") {
        None => {
            tracing::info!("SMTP_HOST isn't set: emails are only logged");
            Mailer::Log
        }
        Some(host) => {
            let encryption =
                Encryption::parse(&setting("SMTP_ENCRYPTION").unwrap_or("tls".into()))?;
            let port = match setting("SMTP_PORT") {
                Some(port) => port
                    .parse()
                    .map_err(|_| format!("SMTP_PORT isn't a port: {port}"))?,
                None if encryption == Encryption::Tls => 465,
                None => 587,
            };
            let login = match (setting("SMTP_USER"), setting("SMTP_PASSWORD_FILE")) {
                (None, _) => None,
                (Some(_), None) => return Err("SMTP_USER needs SMTP_PASSWORD_FILE".into()),
                (Some(user), Some(file)) => {
                    let password = std::fs::read_to_string(&file)
                        .map_err(|error| format!("SMTP_PASSWORD_FILE {file}: {error}"))?;
                    Some((user, password.trim().to_string()))
                }
            };
            tracing::info!(%host, port, ?encryption, "Sending emails by SMTP");
            Mailer::smtp(&host, port, encryption, login).map_err(|error| error.to_string())?
        }
    };
    Ok(Newsletter {
        store: state.subscribers.clone(),
        mailer,
        from,
        owner,
        site_url: state.site_url.clone(),
        posts: state.posts.clone(),
        clock: state.now,
        pause: std::time::Duration::from_secs(2),
    })
}

/// `taichiwayfarer subscribers list|import|remove <email>`: the list of
/// subscribers, on the server through the module's
/// `taichiwayfarer-<name>-subscribers`.
async fn subscribers_command(
    args: &[String],
    database_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fmt::Write as _;
    use std::io::{BufRead, Write};
    let store = Store::new(db::open(database_url).await?);
    let mut output = String::new();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["list"] => {
            output.push_str("email\tsubscribed\tconfirmed\tnote\n");
            for subscriber in store.list().await? {
                writeln!(
                    output,
                    "{}\t{}\t{}\t{}",
                    subscriber.email,
                    subscriber.created_at,
                    subscriber.confirmed_at.as_deref().unwrap_or("no"),
                    subscriber.note.replace(['\n', '\r', '\t'], " "),
                )?;
            }
        }
        ["import"] => {
            let (mut added, mut existing) = (0, 0);
            for line in std::io::stdin().lock().lines() {
                let line = line?;
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let email = parse_email(line).ok_or(format!("not an email address: {line}"))?;
                if store.import(&email, Utc::now()).await? {
                    added += 1;
                } else {
                    existing += 1;
                }
            }
            writeln!(
                output,
                "Added {added}, {existing} were on the list already. The site sends them a welcome email within a minute."
            )?;
        }
        ["remove", email] => {
            if store.remove(email).await? {
                output.push_str("Removed.\n");
            } else {
                return Err(format!("{email} isn't on the list").into());
            }
        }
        _ => {
            return Err(concat!(
                "usage: taichiwayfarer subscribers list | import < emails.txt | remove <email>\n",
                "import adds the addresses, one a line, as confirmed and sends them a welcome email"
            )
            .into());
        }
    }
    std::io::stdout().write_all(output.as_bytes())?;
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
    use chrono::TimeZone;
    use tower::ServiceExt;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    /// The app as it was on November 1, 2026, so a newly published post,
    /// always later than that, cannot change what the tests see.
    async fn test_app(site_url: &str, noindex: bool) -> TestResult<Router> {
        test_app_at(site_url, noindex, november_1).await
    }

    /// After "Don't Think — Just Do" and before "Study Your Movements".
    fn november_1() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 1, 12, 0, 0)
            .single()
            .unwrap_or_default()
    }

    /// The app as if the time were what `now` returns.
    async fn test_app_at(
        site_url: &str,
        noindex: bool,
        now: fn() -> DateTime<Utc>,
    ) -> TestResult<Router> {
        test_app_with(site_url, noindex, now, Store::in_memory().await?)
    }

    /// The app with the given subscribers.
    fn test_app_with(
        site_url: &str,
        noindex: bool,
        now: fn() -> DateTime<Utc>,
        subscribers: Store,
    ) -> TestResult<Router> {
        let root = FsPath::new(env!("CARGO_MANIFEST_DIR"));
        assets::init(&root.join("dist"));
        let state = AppState {
            posts: Arc::new(PostRepository::new(
                &root.join("posts"),
                &root.join("dist"),
            )?),
            site_url: site_url.into(),
            noindex,
            now,
            subscribers,
            wake: Arc::new(Notify::new()),
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
        let app = test_app("https://alt.example.com", false).await?;

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
        let app = test_app("https://www.example.com", false).await?;
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
        let app = test_app("https://www.example.com", false).await?;
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
        let app = test_app("https://alt.example.com", false).await?;
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
        let app = test_app("https://www.example.com", false).await?;
        let (_, index) = fetch(&app, "/").await?;
        assert!(index.contains(r#"<time datetime="2026-10-24">October 24, 2026</time>"#));
        let (_, post) = fetch(&app, "/post/the-slow-set").await?;
        assert!(post.contains(r#"Published <time datetime="2026-08-18">August 18, 2026</time>"#));
        Ok(())
    }

    #[tokio::test]
    async fn pages_have_header_and_main_regions() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
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
        let app = test_app("https://www.example.com", false).await?;
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
        let app = test_app("https://www.example.com", false).await?;
        let (_, page) = fetch(&app, "/search?q=sword").await?;
        assert!(page.contains("<main><h2>Search</h2><figure class=\"quote\">"));
        assert!(page.contains("Without going outside, you may know the whole world."));
        Ok(())
    }

    #[tokio::test]
    async fn search_page_shows_photo_of_first_result_with_one() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        let photo = |page: &str| {
            page.split("<img ")
                .find(|tag| tag.starts_with(r#"class="index""#))
                .map(|tag| tag.split('>').next().unwrap_or_default().to_string())
        };
        // The first result, "Don't Think — Just Do", has a photo.
        let (_, page) = fetch(&app, "/search?q=sword").await?;
        let tag = photo(&page).ok_or("no photo")?;
        assert!(
            tag.contains(r#"src="/static/img/2026-utah-canyon.jpg" alt="Canyons in Utah" width=""#),
            "{tag}"
        );
        // "The Slow Set" comes first but has no photo, so the next result's.
        let (_, page) = fetch(&app, "/search?q=single+whip").await?;
        let tag = photo(&page).ok_or("no photo")?;
        assert!(tag.contains("2025-november-copenhagen.jpg"), "{tag}");
        // Nothing found: the photo of the post list.
        let (_, page) = fetch(&app, "/search?q=qwertyuiop").await?;
        let tag = photo(&page).ok_or("no photo")?;
        assert!(tag.contains("2023-to-the-states.jpg"), "{tag}");
        Ok(())
    }

    #[tokio::test]
    async fn every_page_has_search_in_top_bar() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
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
    async fn navigations_are_named() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        let (_, post) = fetch(&app, "/post/the-slow-set").await?;
        assert!(post.contains(r#"<nav aria-label="Main">"#));
        assert!(post.contains(r#"<nav class="more-stories" aria-label="More stories">"#));
        assert_eq!(post.matches("<nav").count(), 2);
        Ok(())
    }

    #[tokio::test]
    async fn photos_have_fitting_alt_text() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        // Decoration: an empty alt text, so screen readers skip it.
        let decoration = r#"src="/static/img/2023-to-the-states.jpg" alt="""#;
        let (_, index) = fetch(&app, "/").await?;
        assert!(index.contains(decoration));
        let (_, none) = fetch(&app, "/search?q=qwertyuiop").await?;
        assert!(none.contains(decoration));
        let (_, about) = fetch(&app, "/about").await?;
        assert!(about.contains(
            r#"src="/static/img/profile.jpg" alt="Timo practicing Tai Chi in a garden""#
        ));
        Ok(())
    }

    #[tokio::test]
    async fn chinese_text_is_marked_as_chinese() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        let (_, page) = fetch(&app, "/post/the-slow-set").await?;
        let characters = page
            .matches(r#"<div class="characters" lang="zh">"#)
            .count();
        let pinyin = page
            .matches(r#"<div class="pinyin" lang="zh-Latn-pinyin">"#)
            .count();
        assert!(characters > 50, "{characters}");
        assert_eq!(characters, pinyin);
        assert_eq!(page.matches(r#"<div class="characters">"#).count(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn video_has_a_link_without_javascript() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        let (_, page) = fetch(&app, "/post/the-slow-set").await?;
        assert!(page.contains(concat!(
            r#"<div class="video" data-video-id="mha0hp_cLHE">"#,
            r#"<a class="video-fallback" href="https://www.youtube.com/watch?v=mha0hp_cLHE">"#,
            "Watch the video on YouTube</a></div>"
        )));
        Ok(())
    }

    #[tokio::test]
    async fn videos_are_embedded_from_youtube_nocookie() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        let (_, script) = fetch(&app, "/static/embed-video.js").await?;
        assert!(script.contains("https://www.youtube-nocookie.com/embed/"));
        assert!(!script.contains("https://www.youtube.com/embed/"));
        Ok(())
    }

    /// Ten days before "Don't Think — Just Do", but after
    /// "Learn by Watching", the one before it.
    fn october_14() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 14, 12, 0, 0)
            .single()
            .unwrap_or_default()
    }

    #[test]
    fn dev_mode_shows_scheduled_posts() -> TestResult {
        let root = FsPath::new(env!("CARGO_MANIFEST_DIR"));
        let posts = PostRepository::new(&root.join("posts"), &root.join("dist"))?;
        // In dev mode every post counts as published, however far ahead it is.
        assert_eq!(
            posts.published(clock(true)()).count(),
            posts.post_list.len()
        );
        assert!(clock(true)() > Utc::now() + chrono::Duration::days(365 * 1000));
        // Otherwise the real time decides.
        let real = clock(false)();
        assert!((Utc::now() - real).num_seconds().abs() < 5);
        Ok(())
    }

    #[tokio::test]
    async fn scheduled_post_is_hidden_everywhere() -> TestResult {
        let app = test_app_at("https://www.example.com", false, october_14).await?;
        let scheduled = "/post/dont-think-just-do";

        let (_, index) = fetch(&app, "/").await?;
        assert!(!index.contains(scheduled) && index.contains("/post/learn-by-watching"));
        let (_, feed) = fetch(&app, "/feed.xml").await?;
        assert!(!feed.contains(scheduled));
        let (_, sitemap) = fetch(&app, "/sitemap.xml").await?;
        assert!(!sitemap.contains(scheduled));
        let (_, search) = fetch(&app, "/search?q=sword").await?;
        assert!(!search.contains(scheduled) && search.contains("posts found"));

        let request = Request::builder().uri(scheduled).body(Body::empty())?;
        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        // With a slash, it goes to the same not found page.
        assert_eq!(
            redirect(&app, "/post/dont-think-just-do/").await?,
            Some(scheduled.to_string())
        );
        // The post before it has no "next story" yet.
        let (_, previous) = fetch(&app, "/post/learn-by-watching").await?;
        assert!(!previous.contains("Read the next story?"));
        assert!(previous.contains("Read the previous story?"));
        Ok(())
    }

    /// Where a GET of `uri` is redirected permanently, if it is.
    async fn redirect(app: &Router, uri: &str) -> TestResult<Option<String>> {
        let request = Request::builder().uri(uri).body(Body::empty())?;
        let response = app.clone().oneshot(request).await?;
        if response.status() != StatusCode::PERMANENT_REDIRECT {
            return Ok(None);
        }
        let location = response
            .headers()
            .get(header::LOCATION)
            .ok_or("no location")?;
        Ok(Some(location.to_str()?.to_string()))
    }

    #[tokio::test]
    async fn directory_style_urls_redirect_to_the_page() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        for (uri, page) in [
            ("/about/", "/about"),
            ("/post/the-slow-set/", "/post/the-slow-set"),
            ("/subscribe/", "/subscribe"),
            ("/privacy//", "/privacy"),
            ("/search/?q=sword", "/search?q=sword"),
            ("/no-such-page/", "/no-such-page"),
        ] {
            assert_eq!(redirect(&app, uri).await?, Some(page.to_string()), "{uri}");
        }
        // The front page is "/" itself.
        assert_eq!(fetch_status(&app, "/").await?.0, StatusCode::OK);
        // Never to another site, which `//host` would be.
        for uri in ["//example.org/", "///example.org/"] {
            assert_eq!(redirect(&app, uri).await?, None, "{uri}");
        }
        // Forms are not redirected, as that would send them twice.
        let (status, _) = post_form(&app, "/subscribe/", "email=a%40example.com").await?;
        assert_eq!(status, StatusCode::NOT_FOUND);
        Ok(())
    }

    #[tokio::test]
    async fn icons_are_linked() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
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
        let app = test_app("https://www.example.com", false).await?;
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
    async fn posts_end_with_link_to_all_posts() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        for uri in [
            "/post/the-slow-set",
            "/post/bosco-harvest-2023",
            "/post/dont-think-just-do",
        ] {
            let (_, page) = fetch(&app, uri).await?;
            let all = page
                .find(r#"<p class="reference">Or browse <a href="/">all posts</a>.</p></nav>"#)
                .ok_or(format!("no link to all posts on {uri}"))?;
            let more = page
                .find(r#"<nav class="more-stories" aria-label="More stories">"#)
                .ok_or(format!("no more stories on {uri}"))?;
            assert!(more < all, "{uri}");
            // After the next and previous stories, as the last way on.
            assert!(
                page.rfind("story? <a").is_none_or(|story| story < all),
                "{uri}"
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn story_links_are_absolute() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        let (_, page) = fetch(&app, "/post/the-slow-set").await?;
        assert!(page.contains(r#"Read the next story? <a href="/post/learn-by-watching">"#));
        assert!(page.contains(r#"Read the previous story? <a href="/post/june-2026">"#));
        Ok(())
    }

    #[tokio::test]
    async fn nav_marks_current_page() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        let (_, about) = fetch(&app, "/about").await?;
        assert!(about.contains(r#"<a href="/about" aria-current="page">About</a>"#));
        assert!(about.contains(r#"<a href="/">Posts</a>"#));
        // Keyboard and screen reader order must match the visual order.
        let posts = about.find(">Posts</a>").ok_or("no Posts link")?;
        let about_link = about.find(">About</a>").ok_or("no About link")?;
        let search = about.find(r#"name="q""#).ok_or("no search box")?;
        let kofi = about.find("ko-fi.com").ok_or("no Ko-fi link")?;
        assert!(posts < about_link && about_link < search && search < kofi);
        // From a post, both lead elsewhere and stay ordinary links.
        let (_, post) = fetch(&app, "/post/the-slow-set").await?;
        assert!(!post.contains("aria-current"));
        assert!(post.contains(r#"<a href="/">Posts</a>"#));
        assert!(post.contains(r#"<a href="/about">About</a>"#));
        let (_, search) = fetch(&app, "/search?q=sword").await?;
        assert!(!search.contains("aria-current"));
        Ok(())
    }

    #[tokio::test]
    async fn noindex_blocks_search_engines() -> TestResult {
        let app = test_app("https://alt.example.com", true).await?;

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

    async fn post_form(app: &Router, uri: &str, body: &str) -> TestResult<(StatusCode, String)> {
        let request = Request::builder()
            .method("POST")
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body.to_string()))?;
        let response = app.clone().oneshot(request).await?;
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await?;
        Ok((status, String::from_utf8(body.to_vec())?))
    }

    #[tokio::test]
    async fn subscribe_form_has_a_page_of_its_own() -> TestResult {
        let app = test_app("https://www.example.com", false).await?;
        let (_, index) = fetch(&app, "/").await?;
        assert!(index.contains(r#"<a href="/subscribe">subscribe by email</a>"#));
        assert!(!index.contains("<form class=\"subscribe\"") && !index.contains("mailto:"));

        let (status, page) = fetch_status(&app, "/subscribe").await?;
        assert_eq!(status, StatusCode::OK);
        assert!(page.contains(concat!(
            r#"<main><h2>Subscribe</h2><figure class="quote"><blockquote>"#,
            "A good traveler has no fixed plans and is not intent upon arriving.</blockquote>"
        )));
        assert!(page.contains(
            r#"<img class="index" src="/static/img/2025-autumn-in-finland.jpg" alt="" width=""#
        ));
        assert!(page.contains(r#"<form class="subscribe signup" method="post" action="/subscribe"><fieldset><legend>New posts by email</legend>"#));
        assert!(page.contains(r#"name="email" value="" required autocomplete="email">"#));
        assert!(page.contains(r#"<textarea id="subscribe-note" name="note""#));

        // The privacy notice is linked from there only.
        assert!(page.contains(r#"<a href="/privacy">"#));
        for uri in ["/", "/about", "/post/the-slow-set", "/search?q=sword"] {
            let (_, other) = fetch(&app, uri).await?;
            assert!(!other.contains("/privacy"), "{uri}");
        }
        let (_, sitemap) = fetch(&app, "/sitemap.xml").await?;
        assert!(!sitemap.contains("/privacy"));
        let (_, privacy) = fetch(&app, "/privacy").await?;
        assert!(privacy.contains(concat!(
            r#"<article><h2>Privacy</h2><figure class="quote"><blockquote>"#,
            "Those who know do not speak. Those who speak do not know.</blockquote>"
        )));
        assert!(privacy.contains(
            r#"<img class="index" src="/static/img/2024-sandy-lake.jpg" alt="" width=""#
        ));
        Ok(())
    }

    #[tokio::test]
    async fn subscribing_takes_a_confirmation() -> TestResult {
        let store = Store::in_memory().await?;
        let app = test_app_with("https://www.example.com", false, november_1, store.clone())?;

        let (status, page) = post_form(
            &app,
            "/subscribe",
            "email=+a%40example.com&note=Hi&website=",
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        assert!(
            page.contains("<h2>Check Your Inbox</h2>")
                && page.contains("<strong>a@example.com</strong>")
        );
        assert!(page.contains(r#"<meta name="robots" content="noindex">"#));
        assert!(
            page.contains("<blockquote>Do you have the patience to wait till your mud settles")
        );
        assert!(page.contains(r#"src="/static/img/2026-may-moon.jpg" alt="""#));
        let subscriber = store.list().await?.remove(0);
        assert_eq!(
            (subscriber.email.as_str(), subscriber.note.as_str()),
            ("a@example.com", "Hi")
        );
        assert_eq!(subscriber.confirmed_at, None);

        // Opening the link only shows a button; pressing it confirms.
        let link = format!("/confirm?token={}", subscriber.token);
        let (_, page) = fetch(&app, &link).await?;
        assert!(page.contains(&format!(r#"<form class="subscribe" method="post" action="{link}"><button type="submit">Confirm</button>"#)));
        assert!(store.list().await?[0].confirmed_at.is_none());
        assert!(page.contains("<blockquote>The sage does not hoard."));
        let (status, page) = post_form(&app, &link, "").await?;
        assert_eq!(status, StatusCode::OK);
        assert!(page.contains("<h2>You're Subscribed</h2>"));
        assert!(page.contains(r#"src="/static/img/2025-felton-view.jpg" alt="""#));
        assert!(store.list().await?[0].confirmed_at.is_some());
        // Used again: still subscribed.
        let (_, page) = fetch(&app, &link).await?;
        assert!(page.contains("<h2>You're Subscribed</h2>"));

        let (status, page) = fetch_status(&app, "/confirm?token=nope").await?;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(page.contains("<h2>Link Not Valid</h2>"));
        // Errors are plain.
        assert!(!page.contains("<blockquote>"));
        Ok(())
    }

    async fn fetch_status(app: &Router, uri: &str) -> TestResult<(StatusCode, String)> {
        let request = Request::builder().uri(uri).body(Body::empty())?;
        let response = app.clone().oneshot(request).await?;
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await?;
        Ok((status, String::from_utf8(body.to_vec())?))
    }

    #[tokio::test]
    async fn bad_signups_store_nothing() -> TestResult {
        let store = Store::in_memory().await?;
        let app = test_app_with("https://www.example.com", false, november_1, store.clone())?;
        // A bot fills in the hidden field: thanked, but nothing is kept.
        let (status, page) =
            post_form(&app, "/subscribe", "email=bot%40example.com&website=spam").await?;
        assert_eq!(status, StatusCode::OK);
        assert!(page.contains("Check Your Inbox"));
        // A typo: the form again, with what was written.
        let (status, page) = post_form(&app, "/subscribe", "email=a%40&note=%3Cb%3EHi").await?;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(page.contains(r#"<p class="form-error" role="alert">"#));
        assert!(page.contains(r#"value="a@""#) && page.contains("&lt;b&gt;Hi</textarea>"));
        assert_eq!(store.list().await?.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn unsubscribing_deletes_the_address() -> TestResult {
        let store = Store::in_memory().await?;
        let app = test_app_with("https://www.example.com", false, november_1, store.clone())?;
        store.import("a@example.com", Utc::now()).await?;
        let link = format!("/unsubscribe?token={}", store.list().await?[0].token);
        let (_, page) = fetch(&app, &link).await?;
        assert!(
            page.contains("<strong>a@example.com</strong>")
                && page.contains("Unsubscribe</button>")
        );
        assert_eq!(store.list().await?.len(), 1);
        // As a mail app's one-click unsubscribe sends it.
        let (status, page) = post_form(&app, &link, "List-Unsubscribe=One-Click").await?;
        assert_eq!(status, StatusCode::OK);
        assert!(page.contains("<h2>You're Unsubscribed</h2>"));
        assert!(page.contains(
            "<blockquote>Do your work, then step back. The only path to serenity.</blockquote>"
        ));
        assert!(page.contains(r#"src="/static/img/2025-red-mountain.jpg" alt="""#));
        assert_eq!(store.list().await?.len(), 0);
        // Again: the same, without telling whether the address was there.
        let (_, page) = fetch(&app, &link).await?;
        assert!(page.contains("<h2>You're Unsubscribed</h2>"));
        Ok(())
    }
}
