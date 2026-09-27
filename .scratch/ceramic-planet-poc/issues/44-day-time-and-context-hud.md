# 44 — Show current time and interaction prompts

Status: ready-for-agent
Blocked by: 16, 17

## Scope
Add compact HUD display for day/time and the currently resolved contextual action prompt. This is presentation only; do not add quest journal or general HUD framework.

## Acceptance criteria
- Display tracks clock/day rollover and prompt target changes/clears correctly.
- Prompt label and E/mouse action agree with interaction routing.
- HUD remains readable against world colors.

## Tests
- UI tests for clock updates and prompt state; manual readability check in planet scene.