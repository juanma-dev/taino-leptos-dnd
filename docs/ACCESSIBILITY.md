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
  <div role="status" aria-live="polite" aria-atomic="true" class="taino-sr-only">
    Picked up item 3. Currently in position 3 of 7.
  </div>
  ```
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
