//! Error handling foundation for the silver-telegram platform.
//!
//! This module provides only the `Result` type alias using rootcause.
//! Each crate defines its own domain-specific error types in their own
//! error modules, using rootcause's `.context()` to add layer-appropriate
//! context as errors propagate up the stack.

use rootcause::Report;

/// A Result type alias using rootcause's Report for error handling.
///
/// Each layer adds its own context via `.context()` as errors propagate.
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
