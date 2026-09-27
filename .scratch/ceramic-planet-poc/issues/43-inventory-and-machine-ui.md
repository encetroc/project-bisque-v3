# 43 — Add minimal inventory and machine interaction UI

Status: ready-for-agent
Blocked by: 14, 19, 20, 21

## Scope
Expose inventory slots/stacks and minimal workbench/rack/kiln controls needed to select recipes and move items between inventory and machine slots. Avoid broad menu framework.

## Acceptance criteria
- Player can inspect inventory counts and unique ceramic items.
- Machine UI accurately displays inputs, occupied slots, and remaining game-time jobs.
- UI operations call validated domain actions; failed actions do not mutate state.

## Tests
- UI interaction tests for crafting and machine insertion/removal; verify displayed counts against model.