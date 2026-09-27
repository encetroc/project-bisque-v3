# 29 — Add authored NPC dialogue and friendship values

Status: ready-for-agent
Blocked by: 16, 27

## Scope
Add authored generic/request/relationship line pools and friendship 0–100, with talk +1 and liked gift +5. No generated dialogue or decay.

## Acceptance criteria
- Each NPC has at least five generic, three request, and three relationship lines.
- Talking/gifting updates capped friendship and shows authored dialogue.
- Liked-item rules are explicit data.

## Tests
- Unit tests cover increments/cap and dialogue selection; verify catalog line counts.