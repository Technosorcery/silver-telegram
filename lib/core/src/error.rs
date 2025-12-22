//! Error types for the silver-telegram platform.
//!
//! This module provides the foundation for error handling using the rootcause crate.
//! Domain-specific error types should be added as the architecture is developed.

use rootcause::Report;

/// A Result type alias using rootcause's Report for error handling.
pub type Result<T, C = ()> = std::result::Result<T, Report<C>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_type_works() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.expect("should be ok"), 42);
    }
}
