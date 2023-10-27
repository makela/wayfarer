use perseus::prelude::*;
use sycamore::prelude::*;

fn feedback_page<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        h2 { "Feedback" }
        p { "Would you like to suggest some story for me to tell? Maybe from some past camp or some new camp to visit? Maybe I could try to answer some question on Tai Chi? Give me a line and I'll see what I can do about it." }
        form {
            textarea(autofocus = true)
            button { "Send" }
        }
    }
}

#[engine_only_fn]
fn head(cx: Scope) -> View<SsrNode> {
    view! { cx,
        title { "Feedback - Tai Chi Wayfarer" }
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::build("feedback")
        .view(feedback_page)
        .head(head)
        .build()
}
