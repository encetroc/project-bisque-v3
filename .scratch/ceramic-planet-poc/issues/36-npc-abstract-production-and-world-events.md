# 36 — Simulate abstract NPC production and one-time events

Status: ready-for-agent
Blocked by: 17, 27, 31

## Scope
Update abstract NPC production on day boundaries and emit bounded one-time world events (e.g. carpenter adds a bench to an allowed slot). Do not instantiate daily production counts as physical stock.

## Acceptance criteria
- Daily production is data/state only; physical props appear only from explicit events.
- Each world event applies at most once and uses a valid slot.
- Schedule visuals can hint at work without production simulation.

## Tests
- Day progression tests assert abstract count and one-time event behavior/clutter bound.