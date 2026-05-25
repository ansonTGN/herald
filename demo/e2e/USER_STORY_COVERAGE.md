# Demo User Story Coverage

This file records whether current demo tests fully present their referenced
user stories. A test is not complete coverage unless it demonstrates the
acceptance criteria through user-visible behavior.

## Current Gaps

| Demo file | Related user stories | Current status | Required follow-up |
| --- | --- | --- | --- |
| `live/auth/oauth/us-ru-003-github-oauth-live.e2e.ts` | US-RU-003 | Partial live smoke: GitHub success only | Add normal demo coverage for rejected authorization, hidden unconfigured provider, and email association. Google/Facebook/Apple require explicit product decision before claiming coverage. |
| `regular-user/us-pu-06-wechat-qr-rendering-demo.e2e.ts` | US-PU-06, US-WP-005, US-PA-001, US-PA-002 | Partial demo: points-package WeChat QR rendering only | Add comprehensive points-package purchase demo for package list, selected package details, payment success, points fulfillment, new balance, and no subscription record. |
| `live/billing/wechat-pay/us-wp-005-wechat-qr-live.e2e.ts` | US-WP-005, US-PU-06, US-PA-001, US-PA-002 | Partial live smoke: real WeChat QR rendering only | Add non-live demo coverage for polling, cancel order closure, expired QR, failed payment, and unconfigured provider disabled state. |
| `live/billing/payment-attempt/us-pa-001-creem-checkout-live.e2e.ts` | US-PA-001, US-PA-002, US-PA-003 | Partial live smoke: Creem checkout URL and status transition only | Add payment-attempt demos for WeChat subscription, points package, Stripe, uniqueness, pending/success/failure/expired states, webhook compensation, fulfillment, and idempotency. |

## Known Spec Conflicts

- WeChat QR expiry is inconsistent: `US-WP-005` and `US-PA-*` use 2 hours, while
  `US-PU-06` says 15 minutes. Resolve this before writing expiry assertions.
- WeChat scan instruction language differs between docs and UI/tests. Stories
  expect Chinese text, while current tests assert English UI copy.
