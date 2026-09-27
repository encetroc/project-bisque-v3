# 24 — Pick up placed ceramics back into inventory

Status: ready-for-agent
Blocked by: 14, 16, 23

## Scope
Allow selection/interaction with player-placed ceramics to return them to inventory and remove world instance. Respect inventory capacity.

## Acceptance criteria
- Picked-up object retains exact form/material/glaze/state.
- Full inventory leaves object in world.
- Only eligible player-placed objects can be picked up.

## Tests
- Integration tests for round trip and full-inventory refusal.