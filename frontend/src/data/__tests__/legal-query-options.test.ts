import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  queryKeys,
  toAuthConsentAgreements,
  toRecordConsentRequest,
  legalAgreementsQueryOptions,
  legalAgreementQueryOptions,
  consentStatusQueryOptions,
  legalAdminAgreementsQueryOptions,
  recordConsentMutation,
  deleteAccountMutation,
  publishCustomAgreementMutation,
  revertToDefaultAgreementMutation,
} from '@/data/query-options'
import { QUERY_KEYS } from '@/lib/constants'
import type { LegalAgreementSummary } from '@/lib/api-generated'

vi.mock('@/lib/api-generated/sdk.gen', async (importOriginal) => {
  const original = await importOriginal<typeof import('@/lib/api-generated/sdk.gen')>()
  return {
    ...original,
    listAgreements: vi.fn(),
    getAgreement: vi.fn(),
    getConsentStatus: vi.fn(),
    recordConsent: vi.fn(),
    deleteAccount: vi.fn(),
    adminListAgreements: vi.fn(),
    adminPublishCustom: vi.fn(),
    adminRevertToDefault: vi.fn(),
  }
})

import {
  listAgreements,
  getAgreement,
  getConsentStatus,
  recordConsent,
  deleteAccount,
  adminListAgreements,
  adminPublishCustom,
  adminRevertToDefault,
} from '@/lib/api-generated/sdk.gen'

function makeAgreementSummary(overrides?: Partial<LegalAgreementSummary>): LegalAgreementSummary {
  return {
    agreement_type: 'terms_of_service',
    version_id: 'version-001',
    version_no: 1,
    effective_at: '2026-06-30T00:00:00Z',
    title: 'Terms of Service',
    summary: null,
    ...overrides,
  }
}

describe('legal query keys', () => {
  it('differentiates legal agreements by realm', () => {
    const keyRealm1 = queryKeys.legalAgreements('realm-1')
    const keyRealm2 = queryKeys.legalAgreements('realm-2')
    expect(keyRealm1).not.toEqual(keyRealm2)
    expect(keyRealm1[0]).toBe(QUERY_KEYS.LEGAL_AGREEMENTS)
  })

  it('differentiates agreement detail by type within the same realm', () => {
    const keyTos = queryKeys.legalAgreement('realm-1', 'terms_of_service')
    const keyPrivacy = queryKeys.legalAgreement('realm-1', 'privacy_policy')
    expect(keyTos).not.toEqual(keyPrivacy)
    expect(keyTos[0]).toBe(QUERY_KEYS.LEGAL_AGREEMENT)
  })

  it('differentiates agreement detail by locale within the same realm and type', () => {
    const keyEnglish = queryKeys.legalAgreement('realm-1', 'terms_of_service', 'en')
    const keyChinese = queryKeys.legalAgreement('realm-1', 'terms_of_service', 'zh-CN')
    expect(keyEnglish).not.toEqual(keyChinese)
  })

  it('isolates consent status by realm', () => {
    const key = queryKeys.consentStatus('realm-1')
    expect(key).toEqual([QUERY_KEYS.CONSENT_STATUS, 'realm-1'])
  })

  it('isolates admin agreements by realm', () => {
    const key = queryKeys.legalAdminAgreements('realm-1')
    expect(key).toEqual([QUERY_KEYS.LEGAL_ADMIN_AGREEMENTS, 'realm-1'])
  })
})

describe('toAuthConsentAgreements', () => {
  it('maps snake_case summary fields to camelCase auth retry shape', () => {
    const summaries = [
      makeAgreementSummary({
        agreement_type: 'terms_of_service',
        version_id: 'tos-v2',
      }),
      makeAgreementSummary({
        agreement_type: 'privacy_policy',
        version_id: 'privacy-v3',
      }),
    ]

    const result = toAuthConsentAgreements(summaries)

    expect(result).toEqual([
      { agreementType: 'terms_of_service', versionId: 'tos-v2' },
      { agreementType: 'privacy_policy', versionId: 'privacy-v3' },
    ])
  })

  it('returns empty array for empty input', () => {
    expect(toAuthConsentAgreements([])).toEqual([])
  })
})

describe('toRecordConsentRequest', () => {
  it('maps snake_case summary fields to record consent request body', () => {
    const summaries = [
      makeAgreementSummary({
        agreement_type: 'terms_of_service',
        version_id: 'tos-v2',
      }),
    ]

    const result = toRecordConsentRequest(summaries)

    expect(result).toEqual({
      agreements: [{ agreement_type: 'terms_of_service', version_id: 'tos-v2' }],
    })
  })
})

describe('legalAgreementsQueryOptions', () => {
  beforeEach(() => {
    vi.mocked(listAgreements).mockResolvedValue({
      data: { agreements: [makeAgreementSummary()] },
      error: undefined,
    })
  })

  it('calls listAgreements with correct path params', async () => {
    const options = legalAgreementsQueryOptions('realm-1')
    await options.queryFn()

    expect(listAgreements).toHaveBeenCalledWith({ path: { realmId: 'realm-1' } })
  })

  it('returns agreements array', async () => {
    const options = legalAgreementsQueryOptions('realm-1')
    const result = await options.queryFn()

    expect(result).toEqual({ agreements: [makeAgreementSummary()] })
  })

  it('throws when API returns error', async () => {
    vi.mocked(listAgreements).mockResolvedValue({
      data: undefined,
      error: { message: 'Not found', status: 404 },
    })

    const options = legalAgreementsQueryOptions('realm-1')
    await expect(options.queryFn()).rejects.toThrow('Not found')
  })
})

describe('legalAgreementQueryOptions', () => {
  const detailResponse = {
    ...makeAgreementSummary(),
    content: { en: 'Terms body' },
  }

  beforeEach(() => {
    vi.mocked(getAgreement).mockResolvedValue({
      data: detailResponse,
      error: undefined,
    })
  })

  it('calls getAgreement with correct path params', async () => {
    const options = legalAgreementQueryOptions('realm-1', 'terms_of_service')
    await options.queryFn()

    expect(getAgreement).toHaveBeenCalledWith({
      path: { realmId: 'realm-1', agreementType: 'terms_of_service' },
    })
  })

  it('returns agreement detail', async () => {
    const options = legalAgreementQueryOptions('realm-1', 'terms_of_service')
    const result = await options.queryFn()

    expect(result).toEqual(detailResponse)
  })
})

describe('consentStatusQueryOptions', () => {
  const statusResponse = {
    items: [
      {
        agreement_type: 'terms_of_service' as const,
        current_version_id: 'version-001',
        consented_version_id: 'version-001',
        needs_reconsent: false,
      },
    ],
  }

  beforeEach(() => {
    vi.mocked(getConsentStatus).mockResolvedValue({
      data: statusResponse,
      error: undefined,
    })
  })

  it('calls getConsentStatus with correct path params', async () => {
    const options = consentStatusQueryOptions('realm-1')
    await options.queryFn()

    expect(getConsentStatus).toHaveBeenCalledWith({ path: { realmId: 'realm-1' } })
  })

  it('returns consent status response', async () => {
    const options = consentStatusQueryOptions('realm-1')
    const result = await options.queryFn()

    expect(result).toEqual(statusResponse)
  })
})

describe('legalAdminAgreementsQueryOptions', () => {
  const adminResponse = {
    agreements: [
      {
        agreement_type: 'terms_of_service' as const,
        source: 'default' as const,
        current_version: makeAgreementSummary(),
        history: [],
      },
    ],
  }

  beforeEach(() => {
    vi.mocked(adminListAgreements).mockResolvedValue({
      data: adminResponse,
      error: undefined,
    })
  })

  it('calls adminListAgreements with correct path params', async () => {
    const options = legalAdminAgreementsQueryOptions('realm-1')
    await options.queryFn()

    expect(adminListAgreements).toHaveBeenCalledWith({ path: { realmId: 'realm-1' } })
  })

  it('returns admin agreements response', async () => {
    const options = legalAdminAgreementsQueryOptions('realm-1')
    const result = await options.queryFn()

    expect(result).toEqual(adminResponse)
  })
})

describe('recordConsentMutation', () => {
  beforeEach(() => {
    vi.mocked(recordConsent).mockResolvedValue({
      data: undefined,
      error: undefined,
    })
  })

  it('calls recordConsent with realm and request body', async () => {
    const request = toRecordConsentRequest([makeAgreementSummary()])
    await recordConsentMutation('realm-1', request)

    expect(recordConsent).toHaveBeenCalledWith({
      path: { realmId: 'realm-1' },
      body: request,
    })
  })

  it('throws when API returns error', async () => {
    vi.mocked(recordConsent).mockResolvedValue({
      data: undefined,
      error: { message: 'Conflict', status: 409 },
    })

    const request = toRecordConsentRequest([makeAgreementSummary()])
    await expect(recordConsentMutation('realm-1', request)).rejects.toEqual({
      message: 'Conflict',
      status: 409,
    })
  })
})

describe('deleteAccountMutation', () => {
  beforeEach(() => {
    vi.mocked(deleteAccount).mockResolvedValue({
      data: undefined,
      error: undefined,
    })
  })

  it('calls deleteAccount with password body', async () => {
    await deleteAccountMutation({ password: 'secret' })

    expect(deleteAccount).toHaveBeenCalledWith({ body: { password: 'secret' } })
  })

  it('throws when API returns error', async () => {
    vi.mocked(deleteAccount).mockResolvedValue({
      data: undefined,
      error: { message: 'Unauthorized', status: 401 },
    })

    await expect(deleteAccountMutation({ password: 'wrong' })).rejects.toEqual({
      message: 'Unauthorized',
      status: 401,
    })
  })
})

describe('publishCustomAgreementMutation', () => {
  const publishResponse = {
    version_id: 'version-new',
    version_no: 2,
    effective_at: '2026-06-30T00:00:00Z',
  }

  beforeEach(() => {
    vi.mocked(adminPublishCustom).mockResolvedValue({
      data: publishResponse,
      error: undefined,
    })
  })

  it('calls adminPublishCustom with path and body', async () => {
    const body = { content: { en: 'Custom terms' } }
    const result = await publishCustomAgreementMutation('realm-1', 'terms_of_service', body)

    expect(adminPublishCustom).toHaveBeenCalledWith({
      path: { realmId: 'realm-1', agreementType: 'terms_of_service' },
      body,
    })
    expect(result).toEqual(publishResponse)
  })
})

describe('revertToDefaultAgreementMutation', () => {
  const revertResponse = {
    version_id: 'version-reverted',
    version_no: 3,
    effective_at: '2026-06-30T00:00:00Z',
  }

  beforeEach(() => {
    vi.mocked(adminRevertToDefault).mockResolvedValue({
      data: revertResponse,
      error: undefined,
    })
  })

  it('calls adminRevertToDefault with path params', async () => {
    const result = await revertToDefaultAgreementMutation('realm-1', 'terms_of_service')

    expect(adminRevertToDefault).toHaveBeenCalledWith({
      path: { realmId: 'realm-1', agreementType: 'terms_of_service' },
    })
    expect(result).toEqual(revertResponse)
  })
})
