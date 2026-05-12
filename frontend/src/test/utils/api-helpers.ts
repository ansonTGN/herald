/**
 * Helper utilities for API response testing
 */

/**
 * Asserts that an API response was successful and returns the data.
 * Throws if the response contains an error.
 *
 * @param response - The API response object with optional data and error fields
 * @returns The response data (typed)
 * @throws AssertionError if the response contains an error
 *
 * @example
 * ```ts
 * const response = await getRealmConfig({ realmId: 'test' })
 * const data = expectApiSuccess(response)
 * expect(data.registrationBonusPoints).toBe(1000)
 * ```
 */
export function expectApiSuccess<T>(response: { data?: T; error?: any }): T {
  if (response.error) {
    throw new Error(`API call failed: ${JSON.stringify(response.error)}`)
  }
  if (!response.data) {
    throw new Error('API response missing data field')
  }
  return response.data
}
