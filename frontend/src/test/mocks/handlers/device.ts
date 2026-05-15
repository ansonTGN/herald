import { http, HttpResponse } from 'msw'

const API_BASE_URL = 'http://localhost:3000'

// ===== Default Success Responses =====

const DEFAULT_VERIFY_RESPONSE = {
  client_app_name: 'Test App',
  client_app_icon_url: null,
}

const DEFAULT_CONFIRM_RESPONSE = {
  status: 'approved',
}

// ===== Verify Handlers =====

export const deviceVerifyHandler = http.post(
  `${API_BASE_URL}/api/device/:realmId/verify`,
  async () => {
    return HttpResponse.json(DEFAULT_VERIFY_RESPONSE)
  }
)

// ===== Confirm Handlers =====

export const deviceConfirmHandler = http.post(
  `${API_BASE_URL}/api/device/:realmId/confirm`,
  async () => {
    return HttpResponse.json(DEFAULT_CONFIRM_RESPONSE)
  }
)

// ===== Export Handlers Array =====

export const deviceHandlers = [deviceVerifyHandler, deviceConfirmHandler]

// ===== Error Scenario Helpers =====

export function createVerifyErrorHandler(status: number, error: string, errorDescription: string) {
  return http.post(`${API_BASE_URL}/api/device/:realmId/verify`, () => {
    return HttpResponse.json({ error, error_description: errorDescription }, { status })
  })
}

export function createConfirmErrorHandler(status: number, error: string, errorDescription: string) {
  return http.post(`${API_BASE_URL}/api/device/:realmId/confirm`, () => {
    return HttpResponse.json({ error, error_description: errorDescription }, { status })
  })
}
