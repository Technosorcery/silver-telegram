//! silver-telegram web server and UI.
//!
//! This crate provides the Leptos-based web interface for the
//! silver-telegram autonomous personal assistant platform.

#![allow(non_snake_case)]

pub mod app;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::App;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
