/**
 * External API Helper for API Key Authenticated Requests
 *
 * Provides functions for making HTTP requests to the external API (/api/ext/...)
 * using X-API-Key header authentication.
 *
 * The ext API is protected by API Key middleware that validates the key
 * and injects Identity::ThirdParty with associated role permissions.
 *
 * @see backend/api-ext/src/api_key_auth.rs
 * @see backend/api-ext/src/lib.rs
 */

/**
 * Base URL for external API endpoints
 *
 * Uses API_BASE_URL if set, otherwise derives from BASE_URL (frontend URL)
 * by replacing the port with 8080 (backend default), or falls back to localhost:8080.
 */
const rawBaseUrl =
  process.env.API_BASE_URL ||
  process.env.BASE_URL?.replace(/:\d+/, ':8080') ||
  'http://localhost:8080'
export const API_EXT_BASE_URL = rawBaseUrl + '/api/ext'

/**
 * Common request headers for API Key authentication
 */
function authHeaders(apiKey: string): Record<string, string> {
  return {
    'X-API-Key': apiKey,
    'Accept': 'application/json',
  }
}

/**
 * Make an authenticated request to the external API using an API Key
 *
 * Uses Node.js fetch directly to bypass any Playwright proxy interference.
 *
 * @param options Request options
 * @param options.apiKey The API key value for X-API-Key header
 * @param options.method HTTP method (GET, POST, PUT, DELETE)
 * @param options.path URL path after /api/ext (e.g., '/realms')
 * @param options.body Optional JSON body for POST/PUT requests
 * @returns Response status and parsed body
 *
 * @example
 * ```typescript
 * const { status, body } = await makeExtApiRequest({
 *   apiKey: 'herk_abc123...',
 *   method: 'GET',
 *   path: '/realms',
 * })
 * ```
 */
export async function makeExtApiRequest(options: {
  apiKey: string
  method: string
  path: string
  body?: unknown
}): Promise<{ status: number; body: unknown }> {
  const { apiKey, method, path, body } = options
  const url = `${API_EXT_BASE_URL}${path}`
  const headers = authHeaders(apiKey)
  if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
  }

  const response = await fetch(url, {
    method,
    headers,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })

  let responseBody: unknown
  const contentType = response.headers.get('content-type') || ''
  if (contentType.includes('application/json')) {
    responseBody = await response.json().catch(() => response.text())
  } else {
    responseBody = await response.text()
  }

  return {
    status: response.status,
    body: responseBody,
  }
}

/**
 * Make a GET request to the external API with API Key authentication
 *
 * @param apiKey The API key value
 * @param path URL path after /api/ext
 * @returns HTTP status code
 *
 * @example
 * ```typescript
 * const status = await callExtApiWithApiKey('herk_abc123...', '/realms')
 * expect(status).toBe(200)
 * ```
 */
export async function callExtApiWithApiKey(
  apiKey: string,
  path: string
): Promise<number> {
  const result = await makeExtApiRequest({
    apiKey,
    method: 'GET',
    path,
  })
  return result.status
}

/**
 * Make a POST request to the external API with API Key authentication
 *
 * @param apiKey The API key value
 * @param path URL path after /api/ext
 * @param body Request body
 * @returns HTTP status code and response body
 *
 * @example
 * ```typescript
 * const { status, body } = await postExtApiWithApiKey(
 *   'herk_abc123...',
 *   '/permission/check',
 *   { session_token: '...', permission: 'users:read' }
 * )
 * ```
 */
export async function postExtApiWithApiKey(
  apiKey: string,
  path: string,
  body: unknown
): Promise<{ status: number; body: unknown }> {
  return makeExtApiRequest({
    apiKey,
    method: 'POST',
    path,
    body,
  })
}
