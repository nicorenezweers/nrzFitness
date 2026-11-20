use dioxus::prelude::*;

use crate::components::hooks::use_theme_mode::use_theme_provider;
use crate::components::layout::app_bottom_nav::AppBottomNav;
use crate::components::layout::app_wrapper::AppWrapper;
use crate::components::layout::more_sheet::MoreSheet;
use crate::components::layout::profile_gate::ProfileGate;
use crate::components::layout::quick_sheet::QuickSheet;
use crate::components::layout::sidebar::Sidebar;
use crate::components::ui::toast::Toast;
use crate::domain::body::page_body::BodyPage;
use crate::domain::cycle::page_cycle::CyclePage;
use crate::domain::fasting::page_fasting::FastingPage;
use crate::domain::foods::food_sheet::FoodSheet;
use crate::domain::foods::page_foods::FoodsPage;
use crate::domain::goals::page_goals::GoalsPage;
use crate::domain::meds::page_meds::MedsPage;
use crate::domain::reports::page_reports::ReportsPage;
use crate::domain::settings::page_settings::SettingsPage;
use crate::domain::today::page_today::TodayPage;
use crate::domain::training::page_training::TrainingPage;
use crate::state::AppCtx;
use crate::util::today;

const TAILWIND_CSS: Asset = asset!("/public/tailwind.css");
const FAVICON: Asset = asset!("/public/icons/favicon-32x32.png");
const APPLE_ICON: Asset = asset!("/public/icons/apple-touch-icon.png");
const MANIFEST: Asset = asset!("/public/manifest.webmanifest");

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppLayout)]
        #[route("/")]
        Today {},
        #[route("/lebensmittel")]
        Foods {},
        #[route("/training")]
        Training {},
        #[route("/koerper")]
        Body {},
        #[route("/fasten")]
        Fasting {},
        #[route("/medikamente")]
        Meds {},
        #[route("/zyklus")]
        Cycle {},
        #[route("/ziele")]
        Goals {},
        #[route("/berichte")]
        Reports {},
        #[route("/einstellungen")]
        Settings {},
    #[end_layout]
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}

#[component]
pub fn App() -> Element {
    use_theme_provider();

    rsx! {
        document::Title { "nrzFitness" }
        // Dunkles Thema vor dem ersten Zeichnen setzen (kein weisses Aufblitzen).
        document::Script { "if (localStorage.getItem('darkmode') !== 'false') document.documentElement.classList.add('dark');" }
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "apple-touch-icon", href: APPLE_ICON }
        document::Link { rel: "manifest", href: MANIFEST }
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover, interactive-widget=resizes-content" }
        document::Meta { name: "theme-color", content: "#1c1c1c" }
        document::Meta { name: "apple-mobile-web-app-capable", content: "yes" }
        document::Meta { name: "mobile-web-app-capable", content: "yes" }
        document::Meta { name: "apple-mobile-web-app-status-bar-style", content: "black-translucent" }
        document::Meta { name: "apple-mobile-web-app-title", content: "nrzFitness" }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        Router::<Route> {}
    }
}

#[component]
fn AppLayout() -> Element {
    use_context_provider(|| AppCtx {
        profile: Signal::new(None),
        profiles: Signal::new(Vec::new()),
        refresh: Signal::new(0),
        toast: Signal::new(None),
        food_sheet: Signal::new(None),
        more_open: Signal::new(false),
        quick_open: Signal::new(false),
        date: Signal::new(today()),
    });
    rsx! {
        AppWrapper {
            ProfileGate {
                div { class: "flex h-full min-h-0 flex-1",
                    Sidebar {}
                    main { class: "flex min-w-0 flex-1 flex-col overflow-y-auto overflow-x-clip pb-[calc(env(safe-area-inset-bottom)+5rem)] lg:pb-0",
                        div { class: "mx-auto flex w-full max-w-5xl flex-col", Outlet::<Route> {} }
                    }
                }
                AppBottomNav {}
                MoreSheet {}
                QuickSheet {}
                FoodSheet {}
            }
            Toast {}
        }
    }
}

#[component]
fn Today() -> Element {
    rsx! { TodayPage {} }
}
#[component]
fn Foods() -> Element {
    rsx! { FoodsPage {} }
}
#[component]
fn Training() -> Element {
    rsx! { TrainingPage {} }
}
#[component]
fn Body() -> Element {
    rsx! { BodyPage {} }
}
#[component]
fn Fasting() -> Element {
    rsx! { FastingPage {} }
}
#[component]
fn Meds() -> Element {
    rsx! { MedsPage {} }
}
#[component]
fn Cycle() -> Element {
    rsx! { CyclePage {} }
}
#[component]
fn Goals() -> Element {
    rsx! { GoalsPage {} }
}
#[component]
fn Reports() -> Element {
    rsx! { ReportsPage {} }
}
#[component]
fn Settings() -> Element {
    rsx! { SettingsPage {} }
}

#[component]
fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "flex h-full flex-col items-center justify-center gap-4 p-8",
            p { class: "text-2xl font-semibold", "404" }
            p { class: "text-muted-foreground", "Seite nicht gefunden: /{route.join(\"/\")}" }
            Link { to: Route::Today {}, class: "text-sm underline", "Zur Übersicht" }
        }
    }
}
