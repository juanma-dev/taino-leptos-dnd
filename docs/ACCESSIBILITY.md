# Accessibility plan

Accessibility is a Stage-2 deliverable but it shapes Stage-1 decisions. This document
records the target behavior so we don't paint ourselves into a corner now.

## Targets

- **Keyboard-only operation:** any drag achievable with a mouse must be achievable
  with the keyboard alone.
- **Screen-reader announcements** on every state transition (pickup, move, drop, cancel).
- **Reduced-motion respect:** when `prefers-reduced-motion: reduce` is set, all
  animations collapse to 0 ms.
- **Visible focus:** the active draggable always has a visible focus ring; user CSS
  can theme it but cannot remove it.

## Keyboard model (Stage 2)

| Key            | When idle                 | When dragging                  |
| -------------- | ------------------------- | ------------------------------ |
| Tab            | Move focus normally       | Move focus normally (no abort) |
| Space / Enter  | Pick up the focused item  | Drop at current location       |
| Arrow keys     | (Pass through to content) | Move to adjacent droppable     |
| Escape         | (no-op)                   | Cancel drag, restore position  |

## ARIA strategy

We use the **descriptive** pattern, not the legacy `aria-grabbed` (deprecated):

- Each draggable: `role="button"`, `aria-roledescription="draggable item"`,
  `aria-describedby="dnd-instructions"`.
- A visually hidden `<div id="dnd-instructions">` lists the keyboard model.
- A single ARIA live region per `DndContext`:
  ```html
  <div role="alert" aria-live="assertive" aria-atomic="true" class="taino-sr-only">
    Picked up item 3. Currently in position 3 of 7.
  </div>
  ```
  `assertive` (not `polite`) is deliberate: the focus-change announcement
  emitted by the screen reader when the user `Tab`s onto a card and then
  immediately presses `Space` was observed in NVDA to drop the polite
  pickup message that came in behind it. Interrupting the focus
  announcement is the lesser evil — during a drag the user wants the
  drag feedback, not the card's full role description.
- Announcement strings are user-customizable (default English, callbacks for i18n).

## Why not `aria-grabbed` / `aria-dropeffect`?

Deprecated in ARIA 1.1, with no replacement. Screen readers ignored them in
practice. The descriptive pattern (live region + roledescription) is what
`react-beautiful-dnd` settled on after extensive user testing — we follow.

## Touch considerations

- Long-press to pick up (250 ms, configurable) so a tap-and-scroll gesture isn't
  hijacked.
- We `event.preventDefault()` on `touchstart` *only after* the long-press threshold.

## What we won't do

- We won't try to enumerate every assistive technology. NVDA + VoiceOver are the
  baseline. Others may work; we won't add per-AT hacks.
- We won't claim WCAG 2.2 AA compliance until we have a third-party audit.

---

## Stage 2 screen-reader smoke test

Run this checklist before tagging a Stage 2 release. The target is
[`examples/kanban`](../examples/kanban) because it exercises both same-column
reordering and cross-column moves, which is where announcements are most
likely to drift.

### Setup

- **Windows:** install NVDA (`winget install --id NVAccess.NVDA -e`) and start
  it before opening the page.
- **macOS:** VoiceOver, `Cmd+F5` to toggle.
- **Browser:** Chrome or Firefox. (Safari for VoiceOver only.)
- Serve the example: `cd examples/kanban && trunk serve --open`.

### Checklist

For each item, listen for the announcement *exactly once* (no double-fire) and
verify the text matches.

| # | Action | Expected announcement |
| - | ------ | --------------------- |
| 1 | Tab onto the first card | `"Write kanban example, draggable card, button"` (role + roledescription + label) |
| 2 | Press **Space** | `"Picked up item 1. Use arrow keys to move, space or enter to drop, escape to cancel."` |
| 3 | Press **ArrowDown** | `"Item 1 moved over target 2."` |
| 4 | Press **ArrowRight** | `"Item 1 moved over target <neighbor-in-next-column>."` |
| 5 | Press **Space** | `"Dropped item 1 on target <id>."` |
| 6 | Tab onto another card, **Space**, then **Escape** | `"Cancelled drag of item <id>."` and focus restored |
| 7 | While dragging, navigate to a column-tail (Right past the last card) | The card moves but **no announcement** for the tail itself (the tail is `aria-hidden="true"` by design) |

### Pass criteria

- All announcements above fire and the text matches (modulo card ids).
- No spurious announcements during plain Tab navigation or mouse drags
  (mouse drags don't announce — that's intentional, the visual is the
  feedback channel).
- Focus ring stays visible on the active draggable throughout.
- After a cross-column drop, the screen reader can still navigate the
  board with Tab (focus may move to a sibling, which is acceptable until
  we implement focus restoration).

### Recording results

When you run this, append a line to `CHANGELOG.md` under the release section:

```
- Stage 2 screen-reader smoke-test passed: NVDA <version> + Chrome <version>
  on Windows on YYYY-MM-DD.
```

A failed item should land in `docs/ROADMAP.md` as a tracked follow-up, not a
silent regression.
