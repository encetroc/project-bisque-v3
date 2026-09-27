# 26 — Add coins and fixed-price sell transactions

Status: ready-for-agent
Blocked by: 14, 18

## Scope
Add coin balance and fixed prices for selling fired ceramics/resources to merchant interaction. Implement transaction logic only; merchant stock UI is separate.

## Acceptance criteria
- Valid sale removes selected item and credits fixed amount atomically.
- Unfired items or invalid items cannot be sold unless explicit price data exists.
- No dynamic pricing.

## Tests
- Unit tests for prices, balance changes, rejected transactions, and item removal.