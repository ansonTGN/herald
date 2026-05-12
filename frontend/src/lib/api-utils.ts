/**
 * Utility functions for API response handling
 */

/**
 * Handles API responses by checking for errors and throwing if present.
 * This is used to convert the fields-style response ({ data, error, ... })
 * to either throw on error or return the data.
 *
 * @param response - The API response from @hey-api/openapi-ts client
 * @returns The response data
 * @throws The error from the response if present
 */
export function handleApiResponse<T extends { data?: unknown; error?: unknown }>(
  response: T
): NonNullable<T['data']> {
  if (response.error) {
    throw response.error instanceof Error ? response.error : new Error(String(response.error))
  }
  if (!response.data) {
    throw new Error('No data in response')
  }
  return response.data as NonNullable<T['data']>
}
