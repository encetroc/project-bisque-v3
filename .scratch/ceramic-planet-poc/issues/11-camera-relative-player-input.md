# 11 — Make movement camera-relative

Status: ready-for-agent
Blocked by: 08, 10

## Scope
Project camera forward/right onto the player's tangent plane, then map WASD to surface movement. Keep camera yaw independent from movement orientation.

## Acceptance criteria
- W/S move visually forward/back; A/D left/right after arbitrary camera yaw.
- Inputs remain coherent crossing cube seams and poles.
- Degenerate tangent projections are handled without NaN movement.

## Tests
- Automated mapping tests for multiple yaw/normal combinations plus manual camera test route from the spec.