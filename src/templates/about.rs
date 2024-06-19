use perseus::prelude::*;
use sycamore::prelude::*;

fn about_page<G: Html>(cx: Scope) -> View<G> {
    view! { cx,
        article {
            h2 { "About" }
            blockquote { "Be still like a mountain and flow like a river." }
            blockquote { " - Lao Tzu" }
            p { "Hi, I've been practicing Tai Chi Chuan for over two decades. To have this art in my life makes me so lucky, happy and grateful." }
            p {
                "I practice Yang style Tai Chi in "
                a(href = "http://tungtaichi.com") { "Tung school" }
                ". First time I met Grand Master Tung Kai Ying and his son Master Tung Chen Wei in Lapland camp 2005. Since I've followed them across Europe and North America."
            }
            p { "Tai Chi has changed my life in more ways than I could ever have dreamt. It has given me a new family of friends, and it has made me go far further on my travels than I ever could have thought." }
        }
    }
}

#[engine_only_fn]
fn head(cx: Scope) -> View<SsrNode> {
    view! { cx,
        title { "About - Tai Chi Wayfarer" }
    }
}

pub fn get_template<G: Html>() -> Template<G> {
    Template::build("about").view(about_page).head(head).build()
}
