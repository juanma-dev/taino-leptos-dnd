# Keyboard navigation and accessibility

`taino-leptos-dnd` is built so anyone using only a keyboard, or only a
screen reader, can do everything a pointer user can. This guide explains
the model so you can wire it correctly and customize where it matters.

## The keyboard model

| Key            | Idle, focused on a draggable | Mid-drag                       |
| -------------- | ---------------------------- | ------------------------------ |
| Tab            | Move focus normally          | Move focus normally (no abort) |
| Space or Enter | Pick up                      | Drop                           |
| Arrow keys     | (passes through)             | Move over the neighbor droppable |
| Escape         | (passes through)             | Cancel, restore original spot  |

Arrow keys navigate spatially: `ArrowDown` picks the droppable whose center
lies predominantly below the current target, and so on. This works for
vertical lists, horizontal rails, and 2D grids alike.

While dragging, the handler calls `event.prevent_default()` on consumed
keys so they don't double-fire as page-scroll / form-submit / etc.

## Attributes you must set

The hook gives you the behavior. The user-facing **role** is your
responsibility:

```rust
<div
    node_ref=d.node_ref
    tabindex="0"                              // makes it focusable
    role="button"                             // tells AT this is interactive
    aria-roledescription="draggable item"     // overrides "button" verbally
    aria-label="Card 3: Buy milk"             // what the screen reader reads
    on:keydown=move |e| d.on_key_down(&e)
    // ...pointer handlers...
/>
```

- **`tabindex="0"`** — without this, keyboard users can't reach the element.
- **`role="button"`** — interactive sentinel that AT recognizes.
- **`aria-roledescription`** — the modern replacement for the deprecated
  `aria-grabbed`. Screen readers read this *instead* of "button", e.g.
  "Card 3: Buy milk, draggable item".
- **`aria-label`** — short human description of the item. If your visible
  text is already meaningful, you can skip this; otherwise add it.

The hook itself doesn't write attributes onto your element so you stay in
control of markup.

## Announcements

`DndAnnouncer` renders a single `role="status" aria-live="polite"` region.
The library writes to it at every state transition:

```text
Picked up item 3. Use arrow keys to move, space or enter to drop, escape to cancel.
Item 3 moved over target 5.
Item 3 moved over target 7.
Dropped item 3 on target 7.
```

You can override the strings by writing directly to
`ctx.announcement.set("your message")` from your own effect — useful for
localization or for inserting domain context ("Card 'Buy milk' moved to
column 'Done'").

`aria-live="polite"` means the screen reader finishes its current
utterance before announcing — it doesn't interrupt. That's the right
tradeoff for drag-and-drop where many movements happen in quick succession.

## The descriptive pattern (and why not `aria-grabbed`)

Older guides recommended `aria-grabbed` / `aria-dropeffect`. Those were
**deprecated** in ARIA 1.1 with no replacement, and screen readers stopped
implementing them in practice.

The modern pattern is what we use:

1. `aria-roledescription` so AT names the thing correctly.
2. A polite live region for state announcements.
3. Visible focus on the active draggable.

This matches what `react-beautiful-dnd` settled on after extensive user
testing with NVDA and VoiceOver users.

## Reduced motion

`use_flip` (the reorder animation hook) checks
`@media (prefers-reduced-motion: reduce)` and skips the transition when
set. Items still update to their new positions; they just don't slide.

If you write your own transitions (e.g. for a `DragOverlay`), add the
same query to your CSS:

```css
.overlay-card {
    transition: transform 200ms ease-out;
}

@media (prefers-reduced-motion: reduce) {
    .overlay-card { transition: none; }
}
```

## Visible focus

The library doesn't draw a focus ring for you — your design system owns
that. The bare minimum that meets WCAG 2.4.7:

```css
.draggable:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
}
```

`:focus-visible` (not `:focus`) so the ring only appears for keyboard
focus, not mouse clicks. If your visual focus state looks subtle, run it
past someone with low vision or a high-contrast user before shipping.

## What to test before you ship

A short checklist that covers most regressions:

- [ ] Tab onto a draggable, hit Space, arrow once, hit Space. Item moves.
- [ ] Same sequence, hit Escape instead. Item returns to its spot.
- [ ] In dev tools, set `prefers-reduced-motion: reduce`. Reordering still
  works, just without animation.
- [ ] Turn on a screen reader (NVDA on Windows, VoiceOver on macOS / iOS,
  TalkBack on Android). You should hear announcements for pickup, each
  arrow press, and the drop.
- [ ] Zoom to 200% browser zoom. Drags still align correctly (the rects
  recompute on each press).

## What's *not* automatic

- **Reduced-motion in your own animation code.** The library handles its
  own; you handle yours.
- **Live-region politeness arbitration.** If you have multiple live regions
  on the page, `aria-live="polite"` doesn't queue them globally — AT
  vendors pick. Use one announcer if you can.
- **High-contrast / forced-colors mode** (Windows). Test your colors and
  outlines against forced-colors. The library doesn't choose colors for
  you.
