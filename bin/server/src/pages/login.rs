//! Login page component.

use leptos::prelude::*;

/// Login page - redirects to OIDC provider.
#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div class="login-page">
            <div class="login-box">
                <h1>"Log in to silver-telegram"</h1>
                <p>"Click below to authenticate with your identity provider."</p>
                <a href="/auth/login" rel="external" class="login-button">"Log in with SSO"</a>
            </div>
        </div>
    }
}
