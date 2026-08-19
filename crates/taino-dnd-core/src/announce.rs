//! Screen-reader announcement events and the default formatter.
//!
//! Drag-and-drop is invisible to assistive technology unless the app narrates
//! it. The bindings emit an [`AnnounceEvent`] at each lifecycle step (pick up,
//! move, drop, cancel) and run it through a formatter to produce the string
//! pushed into the `aria-live` region.
//!
//! [`default_announcement`] is the library's built-in formatter; it uses raw
//! numeric ids (`"Item 1 moved over target 3."`). Apps almost always want
//! human-readable labels instead — install a custom formatter with
//! `DndContext::set_announcement_formatter` (both bindings) and map the ids to
//! your domain labels there. Keeping the formatter app-side means it composes
//! with i18n and never needs the core to carry label strings.

use std::{fmt::Debug, hash::Hash};

use crate::state::{DraggableId, DroppableId};

/// A drag-lifecycle event worth announcing to assistive technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnounceEvent<T>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
{
    /// The draggable was picked up — a drag began.
    PickedUp {
        /// The draggable that was picked up.
        draggable: DraggableId<T>,
    },
    /// The pointer (or keyboard selection) moved over a droppable, or off all
    /// of them (`over` is `None`).
    MovedOver {
        /// The draggable being moved.
        draggable: DraggableId<T>,
        /// The droppable now under the drag, if any.
        over: Option<DroppableId<T>>,
    },
    /// The draggable was released.
    Dropped {
        /// The draggable that was dropped.
        draggable: DraggableId<T>,
        /// The droppable it landed on, if any.
        over: Option<DroppableId<T>>,
    },
    /// The drag was cancelled (Escape / `pointercancel`), restoring position.
    Cancelled {
        /// The draggable whose drag was cancelled.
        draggable: DraggableId<T>,
    },
}

impl<T> AnnounceEvent<T>
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
{
    /// The draggable this event concerns.
    pub const fn draggable(self) -> DraggableId<T> {
        match self {
            Self::PickedUp { draggable }
            | Self::MovedOver { draggable, .. }
            | Self::Dropped { draggable, .. }
            | Self::Cancelled { draggable } => draggable,
        }
    }
}

/// The library's built-in announcement strings — raw numeric ids.
///
/// Install a custom formatter (`DndContext::set_announcement_formatter`) to
/// produce human-readable, localized messages instead. This is the fallback
/// used when no formatter is set.
#[must_use]
pub fn default_announcement<T>(event: &AnnounceEvent<T>) -> String
where
    T: Debug + Clone + Copy + PartialEq + Eq + Hash + PartialOrd + Ord,
{
    match *event {
        AnnounceEvent::PickedUp { draggable } => format!(
            "Picked up item {:?}. Use arrow keys to move, space or enter to drop, escape to cancel.",
            draggable.0
        ),
        AnnounceEvent::MovedOver { draggable, over: Some(over) } => {
            format!("Item {:?} moved over target {:?}.", draggable.0, over.0)
        }
        AnnounceEvent::MovedOver { draggable, over: None } => {
            format!("Item {:?} is not over a target.", draggable.0)
        }
        AnnounceEvent::Dropped { draggable, over: Some(over) } => {
            format!("Dropped item {:?} on target {:?}.", draggable.0, over.0)
        }
        AnnounceEvent::Dropped { draggable, over: None } => {
            format!("Dropped item {:?} outside any target.", draggable.0)
        }
        AnnounceEvent::Cancelled { draggable } => {
            format!("Cancelled drag of item {:?}.", draggable.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draggable_accessor_covers_all_variants() {
        let d = DraggableId(7);
        let o = Some(DroppableId(3));
        assert_eq!(AnnounceEvent::PickedUp { draggable: d }.draggable(), d);
        assert_eq!(AnnounceEvent::MovedOver { draggable: d, over: o }.draggable(), d);
        assert_eq!(AnnounceEvent::Dropped { draggable: d, over: None }.draggable(), d);
        assert_eq!(AnnounceEvent::Cancelled { draggable: d }.draggable(), d);
    }

    #[test]
    fn default_strings_mention_the_ids() {
        let s = default_announcement(&AnnounceEvent::MovedOver {
            draggable: DraggableId(1),
            over: Some(DroppableId(3)),
        });
        assert!(s.contains('1') && s.contains('3'));
        let s =
            default_announcement(&AnnounceEvent::Dropped { draggable: DraggableId(2), over: None });
        assert!(s.contains('2') && s.to_lowercase().contains("outside"));
    }
}
