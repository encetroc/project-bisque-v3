# 18 — Define ceramic forms, materials, and recipe data

Status: ready-for-agent
Blocked by: 14

## Scope
Define cup, bowl, vase; common/red/pale clay; none/blue/green/white glaze; and distinct greenware/dry/fired item states. No crafting UI or rendering.

## Acceptance criteria
- Recipe inputs/outputs and item variants are data-driven and validated.
- Each object retains form, clay material, glaze, and processing state.
- Invalid stage transitions are rejected.

## Tests
- Unit tests validate catalog completeness and legal/illegal state transitions.