/**
 * Email-OTP Redis Helper for Demo Tests
 *
 * Provides Redis-level functions to read back the one-time login code written
 * by `email_otp::send`. The backend stores the code in Redis as PLAINTEXT
 * (design email-otp-login §4.5 / §5.4 — amended: plaintext rather than hashed,
 * consistent with the password-reset code persisted in the
 * `email_verification_code` table and with the session tokens in the same
 * Redis). The demo environment does not expose a readable mailbox, so the E2E
 * flow reads the code directly from Redis to complete the OTP login UI.
 *
 * The key derivation (`emailotp:{realm_id}:{sha256(normalize_email(email))}`)
 * and the `StoredOtp` JSON shape (`{ code, attempts, max_attempts }`) are
 * reproduced EXACTLY from the backend — verified against
 * `backend/api-auth/src/email_otp.rs` (`otp_redis_key` / `normalize_email` /
 * `StoredOtp`) and the test reference
 * `backend/api/src/tests/helpers/otp_helpers.rs`. Any drift here breaks the
 * demo's ability to read the code.
 *
 * @see ../../../backend/api-auth/src/email_otp.rs (`otp_redis_key`, `normalize_email`, `StoredOtp`)
 * @see ../../../backend/api/src/tests/helpers/otp_helpers.rs (reference derivation)
 * @see ../../../backend/config/demo.toml `[redis] url`
 */

import { createHash } from 'node:crypto'
import Redis from 'ioredis'

const DEFAULT_REDIS_URL = process.env.REDIS_URL || 'redis://127.0.0.1:6379/0'

// A single shared client, mirroring how `reset-password-db-helper.ts`
// constructs its `Pool` once at module scope. The demo suite reuses this
// client across tests; `closeOtpRedis()` tears it down at suite end.
const redisClient = new Redis(DEFAULT_REDIS_URL)

/**
 * Reproduce the backend `normalize_email` (trim + ASCII lowercase). The
 * backend uses Rust `to_ascii_lowercase`; JS `.toLowerCase()` matches for
 * ASCII emails, which is all the demo exercises.
 */
function normalizeEmail(email: string): string {
  return email.trim().toLowerCase()
}

/**
 * Reproduce `email_otp::otp_redis_key`:
 * `emailotp:{realm_id}:{sha256(normalize_email(email))}` (lowercase hex).
 * `realm_id` is inserted verbatim.
 */
function otpRedisKey(realmId: string, email: string): string {
  const digest = createHash('sha256').update(normalizeEmail(email)).digest('hex')
  return `emailotp:${realmId}:${digest}`
}

/**
 * Fetch the most recent OTP code for a (realm, email) pair.
 *
 * Returns `null` when the key is absent (never sent / expired / already
 * consumed one-time) or the stored JSON is unparseable — matches the
 * anti-enumeration behavior of the `send` endpoint, which always returns 200
 * even for unknown emails.
 *
 * @param realmId Realm id, inserted verbatim into the Redis key.
 * @param email   The account email the code was issued for.
 * @returns       The 6-digit code string, or null if none found.
 *
 * @example
 * const code = await getLatestOtpCode('realm-001', 'user@realm-001.com')
 */
export async function getLatestOtpCode(
  realmId: string,
  email: string
): Promise<string | null> {
  const key = otpRedisKey(realmId, email)
  const raw = await redisClient.get(key)

  if (!raw) {
    console.log(`[EmailOtp Redis Helper] No OTP code found for ${email} in realm ${realmId}`)
    return null
  }

  try {
    const stored = JSON.parse(raw) as { code: string; attempts: number; max_attempts: number }
    console.log(
      `[EmailOtp Redis Helper] Found OTP code for ${email} in realm ${realmId} (attempts=${stored.attempts})`
    )
    return stored.code
  } catch {
    console.log(
      `[EmailOtp Redis Helper] Failed to parse OTP payload for ${email} in realm ${realmId}`
    )
    return null
  }
}

/**
 * Delete the stored OTP code for a (realm, email) pair.
 *
 * Use in test cleanup to avoid leaking codes / attempt counters between runs.
 *
 * @param realmId Realm id, inserted verbatim into the Redis key.
 * @param email   The account email to clear the OTP code for.
 *
 * @example
 * await clearOtpCode('realm-001', 'user@realm-001.com')
 */
export async function clearOtpCode(realmId: string, email: string): Promise<void> {
  const key = otpRedisKey(realmId, email)
  await redisClient.del(key)
  console.log(`[EmailOtp Redis Helper] Cleared OTP code for ${email} in realm ${realmId}`)
}

/**
 * Close the shared Redis connection. Call at end of a test suite (mirrors
 * `closeResetPasswordPool`).
 */
export async function closeOtpRedis(): Promise<void> {
  await redisClient.quit()
  console.log('[EmailOtp Redis Helper] Redis connection closed')
}
