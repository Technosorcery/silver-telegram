//! Page components for the application.
//!
//! Each page is a Leptos component that renders a specific route,
//! along with any server functions specific to that page.

pub mod admin;
pub mod home;
pub mod integrations;
pub mod login;
pub mod settings;
pub mod workflow_editor;
pub mod workflows;

// Re-export all page components for convenient access
pub use admin::AdminPage;
pub use home::HomePage;
pub use integrations::IntegrationsPage;
pub use login::LoginPage;
pub use settings::SettingsPage;
pub use workflow_editor::WorkflowEditorPage;
pub use workflows::WorkflowsPage;
