import { http, HttpResponse } from 'msw'

const API_BASE_URL = 'http://localhost:3000'

// ===== Default Success Responses =====

const DEFAULT_REALM_CONFIG = {
  realmId: 'test-realm',
  registrationBonusPoints: 1000,
  freePeriodicPointsAmount: 50,
  freePeriodicGrantPeriodType: 'daily',
  freePeriodicValidityDays: 1,
  createdAt: '2026-03-23T00:00:00Z',
  updatedAt: '2026-03-23T00:00:00Z',
}

// ===== Realm Config Handlers =====

/**
 * Builds a realm-config GET handler that seeds `freePeriodicQuotaWindows`
 * (the design-§3.3/§4.2.2 free-periodic quota windows consumed by
 * `MultiWindowQuotaEditor`). When `quotaWindows` is omitted the response has
 * no `freePeriodicQuotaWindows` key (mirrors a pre-redesign config row).
 */
export function createRealmConfigHandlerWithQuotaWindows(
  quotaWindows?: Array<{ windowSeconds: number; limit: number }>
) {
  return http.get(`${API_BASE_URL}/api/points/:realmId/default-config`, ({ params }) => {
    const { realmId } = params
    return HttpResponse.json({
      ...DEFAULT_REALM_CONFIG,
      realmId: realmId as string,
      ...(quotaWindows ? { freePeriodicQuotaWindows: quotaWindows } : {}),
    })
  })
}

export const getRealmConfigHandler = http.get(
  `${API_BASE_URL}/api/points/:realmId/default-config`,
  ({ params }) => {
    const { realmId } = params

    return HttpResponse.json({
      ...DEFAULT_REALM_CONFIG,
      realmId: realmId as string,
    })
  }
)

export const updateRealmConfigHandler = http.put(
  `${API_BASE_URL}/api/points/:realmId/default-config`,
  async ({ request, params }) => {
    const body = (await request.json()) as any

    if (body.registrationBonusPoints < 0) {
      return HttpResponse.json(
        { message: 'Registration bonus points cannot be negative' },
        { status: 400 }
      )
    }

    if (body.freePeriodicPointsAmount < 0) {
      return HttpResponse.json(
        { message: 'Periodic points amount cannot be negative' },
        { status: 400 }
      )
    }

    if (!['once', 'daily', 'weekly', 'monthly'].includes(body.freePeriodicGrantPeriodType)) {
      return HttpResponse.json({ message: 'Invalid grant period type' }, { status: 400 })
    }

    if (body.freePeriodicValidityDays < 0) {
      return HttpResponse.json({ message: 'Validity days cannot be negative' }, { status: 400 })
    }

    if (body.freePeriodicGrantPeriodType !== 'once' && body.freePeriodicValidityDays < 1) {
      return HttpResponse.json(
        { message: 'Validity days must be at least 1 for non-once periods' },
        { status: 400 }
      )
    }

    return HttpResponse.json({
      ...DEFAULT_REALM_CONFIG,
      realmId: params.realmId as string,
      ...body,
      updatedAt: '2026-03-24T00:00:00Z',
    })
  }
)

// ===== Grant Points Handlers =====

export const grantPointsHandler = http.post(
  `${API_BASE_URL}/api/points/:realmId/grant`,
  async ({ request, params }) => {
    const body = (await request.json()) as any

    return HttpResponse.json({
      transactionId: 'txn-grant-001',
      userId: body.userId,
      amount: body.amount,
      grantedBalance: body.amount,
      totalBalance: body.amount + 500,
      expiresAt: body.validityDays
        ? new Date(Date.now() + body.validityDays * 86400000).toISOString()
        : null,
    })
  }
)

// ===== User Search Handlers =====

const DEFAULT_USERS = [
  {
    id: 'user-1',
    email: 'alice@example.com',
    nickname: 'Alice',
    realmId: 'test-realm',
    status: 1,
    createdAt: '2026-01-01T00:00:00Z',
  },
  {
    id: 'user-2',
    email: 'bob@example.com',
    nickname: null,
    realmId: 'test-realm',
    status: 1,
    createdAt: '2026-01-02T00:00:00Z',
  },
]

export const userSearchHandler = http.get(
  `${API_BASE_URL}/api/users/:realmId`,
  ({ request, params }) => {
    const url = new URL(request.url)
    const email = url.searchParams.get('email') ?? ''
    const filtered = email ? DEFAULT_USERS.filter((u) => u.email.includes(email)) : DEFAULT_USERS

    return HttpResponse.json({
      items: filtered,
      page: 0,
      pageSize: 20,
      total: filtered.length,
    })
  }
)

// ===== Export Handlers Array =====

export const pointsHandlers = [getRealmConfigHandler, updateRealmConfigHandler, grantPointsHandler]

// ===== Error Scenario Helpers =====

export function createRealmConfigErrorHandler(status: number, message: string) {
  return http.put(`${API_BASE_URL}/api/points/:realmId/default-config`, () => {
    return HttpResponse.json({ message }, { status })
  })
}

export function createRealmConfigLoadErrorHandler(status: number, message: string) {
  return http.get(`${API_BASE_URL}/api/points/:realmId/default-config`, () => {
    return HttpResponse.json({ message }, { status })
  })
}

export function createGrantPointsErrorHandler(status: number, message: string) {
  return http.post(`${API_BASE_URL}/api/points/:realmId/grant`, () => {
    return HttpResponse.json({ message }, { status })
  })
}

export function createUserSearchEmptyHandler() {
  return http.get(`${API_BASE_URL}/api/users/:realmId`, () => {
    return HttpResponse.json({
      items: [],
      page: 0,
      pageSize: 20,
      total: 0,
    })
  })
}
