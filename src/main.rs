mod error_views;
mod templates;

use perseus::plugins::Plugins;
use perseus::prelude::*;
use sycamore::prelude::*;

#[perseus::main(perseus_axum::dflt_server)]
pub fn main<G: Html>() -> PerseusApp<G> {
    PerseusApp::new()
        .plugins(Plugins::new().plugin(
            perseus_tailwind::get_tailwind_plugin,
            perseus_tailwind::TailwindOptions {
                in_file: "src/tailwind.css".into(),
                out_file: "dist/styles.css".into(),
            },
        ))
        .static_alias("/favicon.ico", "static/favicon.ico")
        .static_alias("/.perseus/static/styles.css", "dist/styles.css")
        .template(crate::templates::index::get_template())
        .template(crate::templates::about::get_template())
        .template(crate::templates::feedback::get_template())
        .template(crate::templates::post::get_template())
        .error_views(crate::error_views::get_error_views())
        .index_view(|cx| {
            view! { cx,
                head {
                    meta(charset = "utf-8")
                    meta(name = "viewport", content = "width=device-width, initial-scale=1, shrink-to-fit=no")
                    link(rel = "stylesheet", href = "/.perseus/static/styles.css")
                    link(rel = "preconnect", href = "https://fonts.googleapis.com")
                    link(rel = "preconnect", href = "https://fonts.gstatic.com", crossorigin = true)
                    link(href = "https://fonts.googleapis.com/css2?family=Shadows+Into+Light&family=Gloria+Hallelujah&family=Inter&display=swap", rel = "stylesheet")
                }
                body {
                    h1 { a(href = "/") { "Tai Chi Wayfarer" } }
                    nav {
                        ul {
                            li { a(href = "/about") { "About" } }
                            li { a(href = "/feedback") { "Feedback" } }
                        }
                    }
                    PerseusRoot()
                    nav {
                        ul {
                            li { a(href = "/fundme") { "FundMyTravel" } }
                        }
                    }
                }
            }
        })
}
