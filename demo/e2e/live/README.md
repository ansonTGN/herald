# Live Demo Tests

`demo/e2e/live` is only for demo tests that depend on real external services,
real credentials, manual authorization, QR scanning, or public webhook callbacks.
Regular user-story demos that run fully inside the seeded demo environment belong
in the role or domain directories under `demo/e2e/`.

## Index

| File | Related US | Coverage status | External dependency | Manual |
| --- | --- | --- | --- | --- |
| `auth/oauth/us-ru-003-github-oauth-live.e2e.ts` | US-RU-003 | Partial: GitHub success only | GitHub OAuth | Yes |
| `billing/wechat-pay/us-wp-005-wechat-qr-live.e2e.ts` | US-WP-005, US-PU-06, US-PA-001, US-PA-002 | Partial: QR rendering only | WeChat Pay | No for QR rendering |
| `billing/payment-attempt/us-pa-001-creem-checkout-live.e2e.ts` | US-PA-001, US-PA-002, US-PA-003 | Partial: Creem checkout smoke only | Creem | Maybe |
| `billing/payment-attempt/us-pa-001-stripe-checkout-live.e2e.ts` | US-PA-001, US-PA-002, US-PA-003, US-PV-001 | Partial: Stripe checkout smoke only | Stripe | No |
| `billing/one-time-mapping-purchase/us-pu-006-one-time-purchase-live.e2e.ts` | US-PU-006 S1, S2 | Partial: WeChat QR / Stripe redirect initiation only | Stripe / WeChat Pay / Creem | No |

## Rules

- Live tests must declare `Related User Stories`, `Coverage`, `Not Covered`,
  `Live Dependency`, `Manual Step`, `Run Command`, and `Skip/Fail Policy` in the
  file header.
- Live tests are integration smoke tests by default. Do not count them as full
  user-story coverage unless the file explicitly says `Coverage: complete`.
- Live tests may seed and clean up real third-party credentials through API
  helpers when that setup is part of external integration validation.
- Live tests are not part of the default demo regression set unless a command
  explicitly targets them.
- If a test does not require a real external service, real credential, manual
  authorization, QR scan, or public callback URL, do not place it in this
  directory.
