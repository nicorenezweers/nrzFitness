use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn FoodsPage() -> Element {
    rsx! {
        PageHeader { title: "Lebensmittel" }
        PageBody {
            Empty { EmptyTitle { "Lebensmittel" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
