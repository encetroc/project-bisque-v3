# 04 — Sample terrain height and surface normal

Status: ready-for-agent
Blocked by: 01, 02

## Scope
Add bounded tile-authored terrain heights and a shared surface sampling API returning position, height, and normal for a planet coordinate. No procedural noise requirement.

## Acceptance criteria
- Heights stay in a gentle documented range (approximately -0.5 to +1.5).
- Surface sample consistently incorporates radius and terrain height.
- Callers need not implement their own spherical surface math.

## Tests
- Unit tests cover flat and varied tiles, radius/height, and normalized outward normals.