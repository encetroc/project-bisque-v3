# 10 — Add camera yaw and constrained zoom

Status: ready-for-agent
Blocked by: 09

## Scope
Add Q/E or middle-drag yaw around the player’s surface normal and wheel zoom constrained to 10–16 units. No camera collision system.

## Acceptance criteria
- Yaw rotates only around local surface up and remains stable across seams/poles.
- Zoom stays within configured limits; follow behavior remains smooth.
- Controls are documented in the in-game control hints or debug view.

## Tests
- Manual rotation/zoom test while standing and moving at multiple surface locations.