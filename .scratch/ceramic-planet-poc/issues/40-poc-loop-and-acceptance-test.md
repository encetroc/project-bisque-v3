# 40 — Verify the complete first-playthrough loop

Status: ready-for-human
Blocked by: 06, 11, 15, 21, 23, 26, 28, 31, 32, 33, 35, 38, 41, 43, 44, 45

## Scope
Run the complete spec path from first clay gathering through fired sale, Baker request, Highlands/vase, machine upgrade, final bakery upgrade, and persistent player ceramics. Fix only defects required for that path; document evaluation feedback.

## Acceptance criteria
- Fresh save completes the target 30–60 minute path without blockers or data loss.
- Planet circuit crosses seams/poles without control inversion; camera and curvature remain readable.
- Final bakery visibly incorporates player-made objects; no physical production clutter.
- Record tester reactions against the five success criteria and four failure signals.

## Tests
- `cargo test headless_acceptance_path_completes_the_final_order -- --nocapture` runs a headless acceptance-path test through gather/craft, tracked-cup stage transitions and sale, Baker request delivery, Highlands discovery, kiln/workbench upgrades, final-order delivery, and objective completion using production transaction APIs and the objective system. It does not simulate the real-time machine timers or rendered interaction loop.
- `cargo test` runs the full automated suite.
- Human follow-up is still required for the fresh-save 30–60 minute playthrough, save/reload through multi-day progression, spherical seam/pole controls and readability, physical bakery presentation/clutter, and recording reactions against the five success criteria/four failure signals. No manual tester feedback is claimed.