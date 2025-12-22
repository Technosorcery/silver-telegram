//! Main Leptos application component and routing.

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

/// The main application component.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="silver-telegram"/>
        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=path!("/") view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// The home page component.
#[component]
fn HomePage() -> impl IntoView {
    view! {
        <div>
            <h1>"silver-telegram"</h1>
            <p>"Autonomous Personal Assistant Platform"</p>
        </div>
    }
}
