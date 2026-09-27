# 07 — Convert surface locations into transforms

Status: ready-for-agent
Blocked by: 04

## Scope
Implement reusable surface-relative location/altitude to world transform and orientation conversion for actors and props. No movement behavior.

## Acceptance criteria
- Given a planet direction and altitude, compute position and tangent orientation from sampled surface normal.
- API avoids global-Y-up assumptions.
- Supports player, resources, buildings, NPCs, and placed-prop callers.

## Tests
- Unit tests verify outward position and up alignment at multiple non-equatorial locations.