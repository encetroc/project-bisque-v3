# 37 — Add mouse hover, select, and cancel interactions

Status: ready-for-agent
Blocked by: 16, 23

## Scope
Raycast hover/highlight for interactables, left-click select/interact, right-click cancel placement/dialogue, and wheel/Q/E placement rotation support. Do not add click-to-move.

## Acceptance criteria
- Hover identifies/highlights the ray-hit interactable and clears when ray misses.
- Left click routes through the same interaction rules as E; right-click cancels active mode.
- Mouse never controls locomotion.

## Tests
- Input-routing tests and manual checks for hover, interaction, placement rotation, cancel.