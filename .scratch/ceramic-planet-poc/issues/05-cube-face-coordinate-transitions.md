# 05 — Move coordinates across cube-face edges

Status: ready-for-agent
Blocked by: 01

## Scope
Implement `planet.move_coordinate(coord, direction)` including edge transforms and orientation changes. Keep transition logic inside the planet module.

## Acceptance criteria
- A step across each edge resolves to an adjacent face and valid coordinate.
- Orientation is adjusted where face basis changes; gameplay callers do not branch on face pairs.
- Movement remains deterministic.

## Tests
- Unit tests cross every face edge in both directions and verify reversibility/valid bounds.