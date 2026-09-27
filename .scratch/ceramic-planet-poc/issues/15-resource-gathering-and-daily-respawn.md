# 15 — Gather resource nodes and respawn daily

Status: ready-for-agent
Blocked by: 13, 14, 16, 17

## Scope
Connect resource node interaction to inventory; gathered nodes disappear and return at the next day transition. Gathering yields fixed amounts; tools are not required.

## Acceptance criteria
- Interacting with a node grants its specified resources and hides/removes the node.
- Full inventory does not silently destroy uncollected yield.
- Respawn occurs once per day, not on arbitrary time ticks.

## Tests
- Integration test gather → node absent → next-day respawn; test full-inventory behavior.