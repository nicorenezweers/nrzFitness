use dioxus::prelude::*;

use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};

#[component]
pub fn TodayPage() -> Element {
    rsx! {
        PageHeader { title: "Heute" }
        PageBody {
            Empty { EmptyTitle { "Heute" } EmptyDescription { "Kommt gleich." } }
        }
    }
}
