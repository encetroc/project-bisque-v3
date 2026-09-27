# 09 — Add surface-oriented follow camera

Status: ready-for-agent
Blocked by: 07, 08

## Scope
Add pivot-based elevated perspective camera with soft follow and surface-relative up. Initial pitch/distance/FOV can use spec defaults. No rotation or zoom controls.

## Acceptance criteria
- Camera follows player smoothly without direct hard attachment.
- Camera up follows player surface normal at every planet location.
- Player remains readable and curvature is visible.

## Tests
- Manual camera inspection at multiple latitudes and while walking; verify no inversion or global-Y snapping.