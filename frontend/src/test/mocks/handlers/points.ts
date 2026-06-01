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

const DEFAULT_FREE_USER_STATS = {
  totalFreeUsers: 1000,
  activeFreeUsers: 800,
  totalRegistrationBonusGranted: 1000000,
  totalPeriodicPointsGranted: 40000,
  averagePeriodicPointsPerUser: 50,
  upgradeRate: 0.15,
  lastUpdatedAt: '2026-03-23T15:30:00Z',
}

// ===== Realm Config Handlers =====

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

// ===== Free User Statistics Handlers =====

export const getFreeUserStatsHandler = http.get(
  `${API_BASE_URL}/api/points/:realmId/statistics/free-users`,
  ({ request, params }) => {
    const url = new URL(request.url)
    const startDate = url.searchParams.get('startDate')
    const endDate = url.searchParams.get('endDate')

    if (startDate && endDate && startDate > endDate) {
      return HttpResponse.json({ message: 'End date must be after start date' }, { status: 400 })
    }

    return HttpResponse.json({
      ...DEFAULT_FREE_USER_STATS,
    })
  }
)

export const exportFreeUserStatsHandler = http.get(
  `${API_BASE_URL}/api/points/:realmId/statistics/free-users/export`,
  () => {
    const csvData = [
      'Metric,Value',
      'Total Free Users,1000',
      'Active Free Users,800',
      'Total Registration Bonus Granted,1000000',
      'Total Periodic Points Granted,40000',
      'Average Periodic Points Per User,50',
      'Upgrade Rate,0.15',
      'Last Updated,2026-03-23T15:30:00Z',
    ].join('\n')

    return HttpResponse.text(csvData, {
      headers: {
        'Content-Type': 'text/csv',
        'Content-Disposition': 'attachment; filename="free-users-stats.csv"',
      },
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

export const pointsHandlers = [
  getRealmConfigHandler,
  updateRealmConfigHandler,
  getFreeUserStatsHandler,
  exportFreeUserStatsHandler,
  grantPointsHandler,
]

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

export function createFreeUserStatsErrorHandler(status: number, message: string) {
  return http.get(`${API_BASE_URL}/api/points/:realmId/statistics/free-users`, () => {
    return HttpResponse.json({ message }, { status })
  })
}

export function createExportErrorHandler(status: number, message: string) {
  return http.get(`${API_BASE_URL}/api/points/:realmId/statistics/free-users/export`, () => {
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
