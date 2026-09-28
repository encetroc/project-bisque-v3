# 15 — Gather resource nodes and respawn daily

Status: resolved
Blocked by: 13, 14, 16, 17

## Scope
Connect resource node interaction to inventory; gathered nodes disappear and return at the next day transition. Gathering yields fixed amounts; tools are not required.

## Acceptance criteria
- Interacting with a node grants its specified resources and hides/removes the node.
- Full inventory does not silently destroy uncollected yield.
- Respawn occurs once per day, not on arbitrary time ticks.

## Tests
- Integration test gather → node absent → next-day respawn; test full-inventory behavior.

## Answer
Implemented generic resource gathering, hiding gathered nodes until a `DayTransition`, and retaining nodes when inventory cannot accept the yield. Every node grants one unit of its authored resource. Added headless tests for gathering/respawn and full inventory; save/load also persists gathered-node state.

Verified with `cargo test resource_nodes::tests` and `cargo test save_game::tests`.