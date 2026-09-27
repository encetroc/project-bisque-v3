# 08 — Move the player over the spherical surface

Status: ready-for-agent
Blocked by: 05, 07

## Scope
Add controllable capsule movement using planet coordinates and tangent-plane inputs. Include walk/run speeds and keep the player at terrain height; do not implement camera-relative controls yet.

## Acceptance criteria
- Player remains on the surface and rotates with local normal.
- Movement crosses cube seams without teleporting or flipping.
- Walk/run are distinct and bounded.

## Tests
- Automated movement tests cross all face seams and poles; manual continuous circuit check in planet test scene.