# 39 — Fade objects that obstruct the player view

Status: ready-for-agent
Blocked by: 09

## Scope
Apply a simple alpha reduction (about 0.3) to large objects between camera and player, particularly trees. No complex collision camera.

## Acceptance criteria
- Obstruction clears enough visibility to read player position.
- Alpha restores when line of sight is clear; unrelated objects remain opaque.
- No camera repositioning/collision solver.

## Tests
- Manual placement of tree between camera/player and verify fade/restoration.