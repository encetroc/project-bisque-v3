# 32 — Upgrade a requested NPC property visibly

Status: ready-for-agent
Blocked by: 30, 31

## Scope
Add one upgrade state for the Baker property. Require specified fired ceramics and coins; update bakery geometry and slot layout on completion.

## Acceptance criteria
- Upgrade can be completed once only after all requirements are met.
- Bakery visibly changes and retains/redistributes player-delivered objects into valid slots.
- No general building-construction system is introduced.

## Tests
- Integration tests for blocked/valid/repeated upgrade and persistent objects after upgrade.