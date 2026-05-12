//! The drag state machine.
//!
//! ```text
//!                pointerdown
//!   Idle  ─────────────────────────►  Pressed
//!     ▲                                  │
//!     │                                  │  movement exceeds threshold
//!     │                                  ▼
//!     │                              Dragging
//!     │                                  │
//!     │                                  │  pointerup / cancel
//!     │                                  ▼
//!     └─────────────────────────── Dropping (terminal, settles back to Idle)
//! ```
//!
//! The machine is intentionally minimal in Stage 1. Stage 2 will add a
//! keyboard-driven path (Space → Dragging, Esc → cancel).

use crate::{error::Error, geometry::Point};

/// Opaque identifier for a draggable element.
///
/// User code is responsible for keeping IDs unique within a single `DndContext`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DraggableId(pub u64);

/// Opaque identifier for a drop target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DroppableId(pub u64);

/// The current state of a drag interaction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragState {
    /// No drag in progress.
    Idle,
    /// Pointer is down on a draggable but has not yet moved past the threshold.
    Pressed {
        /// The draggable the user pressed on.
        id: DraggableId,
        /// Pointer position when the press started (CSS pixels).
        start: Point,
    },
    /// Pointer has moved past the threshold; this is an active drag.
    Dragging {
        /// The draggable being dragged.
        id: DraggableId,
        /// Pointer position when the press started.
        start: Point,
        /// Current pointer position.
        current: Point,
    },
    /// Drag has been released (or cancelled). Bindings settle visuals before
    /// transitioning back to [`DragState::Idle`].
    Dropping {
        /// The draggable that was dropped.
        id: DraggableId,
    },
}

impl DragState {
    const fn name(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Pressed { .. } => "Pressed",
            Self::Dragging { .. } => "Dragging",
            Self::Dropping { .. } => "Dropping",
        }
    }
}

/// Events that drive [`DragState`] transitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragEvent {
    /// A pointer pressed on a draggable.
    PointerDown {
        /// Draggable that was pressed.
        id: DraggableId,
        /// Pointer position.
        at: Point,
    },
    /// A pointer moved.
    PointerMove {
        /// New pointer position.
        at: Point,
    },
    /// A pointer was released or the drag was otherwise terminated.
    PointerUp,
    /// The drag was cancelled (e.g. `pointercancel`, Escape).
    Cancel,
    /// Bindings finished any drop animation and the state should return to [`DragState::Idle`].
    Settle,
}

impl DragEvent {
    const fn name(self) -> &'static str {
        match self {
            Self::PointerDown { .. } => "PointerDown",
            Self::PointerMove { .. } => "PointerMove",
            Self::PointerUp => "PointerUp",
            Self::Cancel => "Cancel",
            Self::Settle => "Settle",
        }
    }
}

/// Default pixel distance the pointer must travel before `Pressed` becomes `Dragging`.
///
/// 5 CSS pixels matches what most native UI toolkits use to distinguish
/// "click" from "drag".
pub const DEFAULT_DRAG_THRESHOLD: f64 = 5.0;

/// Apply an event to the current state, returning the new state.
///
/// Returns [`Error::InvalidTransition`] when the event does not make sense for
/// the current state (e.g. `PointerUp` while `Idle`).
///
/// The threshold parameter controls when `Pressed` is promoted to `Dragging`.
/// Most callers should pass [`DEFAULT_DRAG_THRESHOLD`].
///
/// # Examples
///
/// ```
/// use taino_dnd_core::{
///     state::{DragEvent, DragState, DraggableId, DEFAULT_DRAG_THRESHOLD, transition},
///     Point,
/// };
///
/// let id = DraggableId(1);
/// let s0 = DragState::Idle;
/// let s1 = transition(s0, DragEvent::PointerDown { id, at: Point::new(0.0, 0.0) }, DEFAULT_DRAG_THRESHOLD).unwrap();
/// assert!(matches!(s1, DragState::Pressed { .. }));
/// ```
// Two arms below resolve to `Idle` (cancel-from-Dragging and settle-from-Dropping).
// They are kept separate on purpose: collapsing them would erase the semantic
// distinction between "user aborted mid-drag" and "drop animation finished".
#[allow(clippy::match_same_arms)]
pub fn transition(state: DragState, event: DragEvent, threshold: f64) -> Result<DragState, Error> {
    match (state, event) {
        // Idle → Pressed
        (DragState::Idle, DragEvent::PointerDown { id, at }) => {
            Ok(DragState::Pressed { id, start: at })
        }

        // Pressed → Dragging (if moved past threshold) or stay Pressed
        (DragState::Pressed { id, start }, DragEvent::PointerMove { at }) => {
            if start.distance_squared(at) >= threshold * threshold {
                Ok(DragState::Dragging { id, start, current: at })
            } else {
                Ok(DragState::Pressed { id, start })
            }
        }

        // Pressed → Idle on release (it was a click, not a drag)
        (DragState::Pressed { .. }, DragEvent::PointerUp | DragEvent::Cancel) => {
            Ok(DragState::Idle)
        }

        // Dragging tracks pointer moves
        (DragState::Dragging { id, start, .. }, DragEvent::PointerMove { at }) => {
            Ok(DragState::Dragging { id, start, current: at })
        }

        // Dragging → Dropping
        (DragState::Dragging { id, .. }, DragEvent::PointerUp) => Ok(DragState::Dropping { id }),

        // Dragging → Idle on cancel
        (DragState::Dragging { .. }, DragEvent::Cancel) => Ok(DragState::Idle),

        // Dropping → Idle once visuals settle
        (DragState::Dropping { .. }, DragEvent::Settle) => Ok(DragState::Idle),

        // Anything else is an invalid transition.
        (state, event) => {
            Err(Error::InvalidTransition { event: event.name(), state: state.name() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: f64 = DEFAULT_DRAG_THRESHOLD;

    #[test]
    fn idle_pointer_down_goes_to_pressed() {
        let id = DraggableId(7);
        let s =
            transition(DragState::Idle, DragEvent::PointerDown { id, at: Point::new(0.0, 0.0) }, T)
                .unwrap();
        assert!(matches!(s, DragState::Pressed { id: DraggableId(7), .. }));
    }

    #[test]
    fn pressed_below_threshold_stays_pressed() {
        let id = DraggableId(1);
        let start = Point::new(10.0, 10.0);
        let s = transition(
            DragState::Pressed { id, start },
            DragEvent::PointerMove { at: Point::new(12.0, 10.0) },
            T,
        )
        .unwrap();
        assert!(matches!(s, DragState::Pressed { .. }));
    }

    #[test]
    fn pressed_above_threshold_becomes_dragging() {
        let id = DraggableId(1);
        let start = Point::new(0.0, 0.0);
        let s = transition(
            DragState::Pressed { id, start },
            DragEvent::PointerMove { at: Point::new(20.0, 0.0) },
            T,
        )
        .unwrap();
        assert!(matches!(s, DragState::Dragging { .. }));
    }

    #[test]
    fn pressed_pointer_up_resets_to_idle() {
        let id = DraggableId(1);
        let s = transition(
            DragState::Pressed { id, start: Point::new(0.0, 0.0) },
            DragEvent::PointerUp,
            T,
        )
        .unwrap();
        assert_eq!(s, DragState::Idle);
    }

    #[test]
    fn dragging_pointer_up_enters_dropping() {
        let id = DraggableId(2);
        let s = transition(
            DragState::Dragging { id, start: Point::new(0.0, 0.0), current: Point::new(50.0, 0.0) },
            DragEvent::PointerUp,
            T,
        )
        .unwrap();
        assert_eq!(s, DragState::Dropping { id });
    }

    #[test]
    fn dragging_cancel_goes_back_to_idle() {
        let id = DraggableId(2);
        let s = transition(
            DragState::Dragging { id, start: Point::new(0.0, 0.0), current: Point::new(50.0, 0.0) },
            DragEvent::Cancel,
            T,
        )
        .unwrap();
        assert_eq!(s, DragState::Idle);
    }

    #[test]
    fn dropping_settles_to_idle() {
        let id = DraggableId(3);
        let s = transition(DragState::Dropping { id }, DragEvent::Settle, T).unwrap();
        assert_eq!(s, DragState::Idle);
    }

    #[test]
    fn invalid_transition_reports_event_and_state() {
        let err = transition(DragState::Idle, DragEvent::PointerUp, T).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("PointerUp"));
        assert!(msg.contains("Idle"));
    }
}
