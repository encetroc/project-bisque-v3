# 16 — Resolve the best nearby interactable

Status: ready-for-agent
Blocked by: 07

## Scope
Implement interaction range/front-facing/nearest-priority selection and contextual prompt data. Bind keyboard E; no individual resource or machine actions beyond a test interactable.

## Acceptance criteria
- Chooses only in-range candidates, preferring roughly forward then nearest among valid targets per documented rule.
- Prompt reflects selected target and clears when none qualifies.
- Pressing E dispatches one selected target only.

## Tests
- Unit tests cover range, direction, nearest tie-break, and no-target cases; manual prompt check.