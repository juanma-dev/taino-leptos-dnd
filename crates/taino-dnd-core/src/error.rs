//! Error types for `taino-dnd-core`.
//!
//! `Error` is `#[non_exhaustive]` so we can add variants without a major bump.

use thiserror::Error;

/// Errors produced by core operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A `DragEvent` was applied to a `DragState` that is not in a compatible variant.
    ///
    /// Example: receiving `DragEvent::PointerUp` while the state is `Idle`.
    #[error("invalid transition: cannot apply {event} while in {state}")]
    InvalidTransition {
        /// Human-readable name of the event that was rejected.
        event: &'static str,
        /// Human-readable name of the state at the time of rejection.
        state: &'static str,
    },
}
