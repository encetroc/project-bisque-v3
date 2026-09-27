# 30 — Define and fulfill NPC ceramic requests

Status: ready-for-agent
Blocked by: 18, 26, 27, 29

## Scope
Represent request chains as authored requirements/rewards and implement requirement checking/atomic delivery. Start with Baker request for two fired cups. Do not yet spawn persistent cups.

## Acceptance criteria
- Request state moves available → active → complete without duplicate reward.
- Delivery consumes only matching finished items and applies friendship/reward effects.
- Insufficient inventory leaves request and inventory unchanged.

## Tests
- Request state/transaction tests for partial, complete, and repeated delivery.