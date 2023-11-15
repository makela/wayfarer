#[cfg(engine)]
mod api;
mod error_views;
#[cfg(engine)]
mod model;
#[cfg(engine)]
mod schema;
mod templates;

use perseus::plugins::Plugins;
use perseus::prelude::*;
use serde::{Deserialize, Serialize};
use sycamore::prelude::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CreateFeedback {
    email: String,
    feedback: String,
}

#[cfg(engine)]
pub async fn dflt_server<
    M: perseus::stores::MutableStore + 'static,
    T: perseus::i18n::TranslationsManager + 'static,
>(
    turbine: &'static perseus::turbine::Turbine<M, T>,
    opts: perseus::server::ServerOptions,
    (host, port): (String, u16),
) {
    use crate::api::{create_feedback, get_connection_pool};
    use diesel::prelude::PgConnection;
    use diesel::r2d2::{ConnectionManager, Pool};
    use perseus_warp::perseus_routes;
    use std::net::SocketAddr;
    use warp::{http::Uri, Filter, Rejection, Reply};

    async fn create_feedback_handler(
        x: CreateFeedback,
        pool: Pool<ConnectionManager<PgConnection>>,
    ) -> Result<impl Reply, Rejection> {
        let mut connection = pool.get().unwrap();
        let feedback = create_feedback(&mut connection, &x.email, &x.feedback).await;
        Ok(warp::reply::json(&feedback.id))
    }

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid address prodived to bind to.");

    let pool = get_connection_pool();
    let perseus_routes = perseus_routes(turbine, opts).await;
    let api_route = warp::post()
        .and(warp::path("feedback"))
        .and(warp::body::form::<CreateFeedback>())
        .and(warp::any().map(move || pool.clone()))
        .and_then(create_feedback_handler)
        .map(|_| warp::redirect::see_other(Uri::from_static("/thankyou")));
    let routes = api_route.or(perseus_routes);
    warp::serve(routes).run(addr).await;
}

#[perseus::main(dflt_server)]
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
        .template(crate::templates::thankyou::get_template())
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
                }
            }
        })
}
