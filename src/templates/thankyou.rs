use perseus::prelude::*;
use sycamore::prelude::*;

fn thankyou_page<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        article {
            h2 { "Feedback" }
            p { "Thank you for your feedback!" }
        }
    }
}

#[engine_only_fn]
fn head(cx: Scope) -> View<SsrNode> {
    view! { cx,
        title { "Thank you - Tai Chi Wayfarer" }
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::build("thankyou")
        .view(thankyou_page)
        .head(head)
        .build()
}
