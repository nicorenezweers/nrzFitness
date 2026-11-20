use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn BodyPage() -> Element {
    rsx! {
        PageHeader { title: "Körper" }
        PageBody {
            Empty { EmptyTitle { "Körper" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
