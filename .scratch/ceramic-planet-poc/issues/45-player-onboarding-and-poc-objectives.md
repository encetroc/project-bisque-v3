# 45 — Guide the player through the initial POC progression

Status: ready-for-agent
Blocked by: 15, 19, 21, 26, 30, 35, 41

## Scope
Add a small authored objective tracker for first clay ×3 → cup craft/dry/fire/sell, Baker request, Highlands discovery, kiln upgrade, and final Baker order. No procedural quests.

## Acceptance criteria
- Objectives unlock in intended order and complete from actual game state/events.
- Current objective is visible and cannot be completed by unrelated actions.
- Progress is persisted through existing save data.

## Tests
- Progression tests cover event ordering, completion, save/load, and no false completion.