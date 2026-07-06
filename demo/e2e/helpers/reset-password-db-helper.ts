/**
 * Reset Password Database Helper for Demo Tests
 *
 * Provides database-level functions to retrieve the password-reset verification
 * code written by `UserService::reset_password_request`. The backend stores the
 * code in `email_verification_code` with type = 'reset_password'; the demo
 * environment does not expose a readable mailbox, so the E2E flow reads the code
 * directly from the database to complete the reset-password UI.
 *
 * @see ../../../backend/domain/src/user/services/basic.rs (reset_password_request/confirm)
 * @see ../../../backend/api-auth/src/reset_password.rs
 */

import { Pool, QueryResult } from 'pg'

const DEFAULT_DATABASE_URL =
  process.env.DATABASE_URL || 'postgres://postgres:postgres@127.0.0.1:5432/herald_demo'

const pool = new Pool({
  connectionString: DEFAULT_DATABASE_URL,
})

const RESET_CODE_TYPE = 'reset_password'

/**
 * Fetch the most recent password-reset code for an email address.
 *
 * Returns `null` when no code exists (matches the backend's anti-enumeration
 * behavior: the request endpoint always returns ok even for unknown emails).
 *
 * @param email The account email the reset link was requested for.
 * @returns The verification code string, or null if none found.
 *
 * @example
 * const code = await getLatestResetCode('user@example.com')
 */
export async function getLatestResetCode(email: string): Promise<string | null> {
  const client = await pool.connect()
  try {
    const result: QueryResult<{ verification_code: string }> = await client.query(
      `SELECT verification_code
       FROM email_verification_code
       WHERE email = $1 AND type = $2
       ORDER BY created_at DESC
       LIMIT 1`,
      [email, RESET_CODE_TYPE]
    )

    if (result.rowCount === 0) {
      console.log(`[ResetPwd DB Helper] No reset code found for ${email}`)
      return null
    }

    return result.rows[0].verification_code
  } finally {
    client.release()
  }
}

/**
 * Delete all password-reset codes for an email address.
 *
 * Use in test cleanup to avoid leaking codes between test runs.
 *
 * @param email The account email to clear reset codes for.
 *
 * @example
 * await clearResetCodes('user@example.com')
 */
export async function clearResetCodes(email: string): Promise<void> {
  const client = await pool.connect()
  try {
    await client.query(
      `DELETE FROM email_verification_code WHERE email = $1 AND type = $2`,
      [email, RESET_CODE_TYPE]
    )
    console.log(`[ResetPwd DB Helper] Cleared reset codes for ${email}`)
  } finally {
    client.release()
  }
}

/**
 * Close the database connection pool. Call at end of a test suite if needed.
 */
export async function closeResetPasswordPool(): Promise<void> {
  await pool.end()
  console.log('[ResetPwd DB Helper] Database connection pool closed')
}
