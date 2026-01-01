//! Home page component.

use crate::user::get_current_user;
use leptos::prelude::*;

/// The home page component.
#[component]
pub fn HomePage() -> impl IntoView {
    let user = Resource::new(|| (), |_| get_current_user());

    view! {
        <div class="home-page">
            <Suspense fallback=move || view! { <p>"Loading..."</p> }>
                {move || {
                    user.get().map(|result| {
                        match result {
                            Ok(Some(user_info)) => {
                                let greeting = user_info.display_name.clone()
                                    .map(|n| format!("Welcome, {}!", n))
                                    .unwrap_or_else(|| "Welcome!".to_string());
                                view! {
                                    <div>
                                        <h1>{greeting}</h1>
                                        <p>"Your autonomous personal assistant is ready."</p>
                                    </div>
                                }.into_any()
                            },
                            Ok(None) => view! {
                                <div>
                                    <h1>"silver-telegram"</h1>
                                    <p>"Autonomous Personal Assistant Platform"</p>
                                    <p>"Please log in to access your assistant."</p>
                                    <a href="/auth/login" rel="external" class="cta-button">"Log in"</a>
                                </div>
                            }.into_any(),
                            Err(_) => view! {
                                <div>
                                    <h1>"silver-telegram"</h1>
                                    <p>"Autonomous Personal Assistant Platform"</p>
                                    <a href="/auth/login" rel="external" class="cta-button">"Log in"</a>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}
