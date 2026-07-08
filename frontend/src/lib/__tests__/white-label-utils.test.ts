import { describe, it, expect } from 'vitest'
import {
  emptyWhiteLabelConfig,
  normalizeWhiteLabelConfig,
  toUpdateWhiteLabelConfigRequest,
} from '../realm-config-utils'

/**
 * Factory: a fully-populated valid white-label form. Overrides merge on top so
 * tests can mutate a single field without re-declaring the whole payload.
 */
function makeFullWhiteLabelConfig(overrides: Record<string, unknown> = {}) {
  return {
    logoUrl: 'https://cdn.example.com/logo.svg',
    accentColor: '#2563eb',
    background: { type: 'gradient' as const, value: 'linear-gradient(135deg, #1e3a8a, #2563eb)' },
    footerText: 'Example Inc.',
    loginTitle: 'Sign in to Example',
    loginSubtitle: 'Use your Example account',
    registerTitle: 'Create your Example account',
    registerSubtitle: 'Start with Example',
    ...overrides,
  }
}

/** The canonical all-null empty form, asserted in one place for reuse below. */
const EMPTY_FORM = {
  logoUrl: null,
  accentColor: null,
  background: null,
  footerText: null,
  loginTitle: null,
  loginSubtitle: null,
  registerTitle: null,
  registerSubtitle: null,
}

describe('emptyWhiteLabelConfig', () => {
  it('returns an all-null form (unconfigured realm)', () => {
    expect(emptyWhiteLabelConfig()).toEqual(EMPTY_FORM)
  })

  // Guards downstream default-equality checks (e.g. form reset) against the
  // helper accidentally sharing one mutated object across calls.
  it('returns a fresh object each call', () => {
    const a = emptyWhiteLabelConfig()
    const b = emptyWhiteLabelConfig()
    expect(a).not.toBe(b)
    a.logoUrl = 'mutated'
    expect(b.logoUrl).toBeNull()
  })
})

describe('normalizeWhiteLabelConfig', () => {
  describe('falls back to empty defaults for unusable input', () => {
    it.each([
      ['null', null],
      ['undefined', undefined],
      ['empty string', ''],
      ['malformed JSON string', '{"logoUrl":'],
      ['non-object JSON', '"just a string"'],
      ['array', [1, 2, 3]],
      ['number', 42],
      ['boolean', true],
    ])('returns all-null form for %s', (_label, input) => {
      expect(normalizeWhiteLabelConfig(input)).toEqual(EMPTY_FORM)
    })
  })

  describe('falls back to empty defaults for missing required fields', () => {
    // Every field is nullable-but-required. A backend payload that omits any
    // field (or carries a non-null invalid value on one) fails safeParse, and
    // normalize returns the *whole* empty form rather than a half-populated
    // one. This is the documented "malformed stored config never crashes the
    // admin form" contract.
    it('returns empty form when a field is missing', () => {
      // Missing `background` and other fields → not a valid form.
      const result = normalizeWhiteLabelConfig({ logoUrl: 'https://x/logo.svg' })

      expect(result).toEqual(EMPTY_FORM)
    })

    it('returns empty form when a field has the wrong type', () => {
      const result = normalizeWhiteLabelConfig({
        ...EMPTY_FORM,
        logoUrl: 12345, // number where string|null expected
      })

      expect(result).toEqual(EMPTY_FORM)
    })

    it('returns empty form when background.type is an invalid enum', () => {
      const result = normalizeWhiteLabelConfig({
        ...EMPTY_FORM,
        background: { type: 'video', value: 'x' },
      })

      expect(result).toEqual(EMPTY_FORM)
    })
  })

  describe('preserves valid input', () => {
    it('preserves a fully populated config', () => {
      const full = makeFullWhiteLabelConfig()

      expect(normalizeWhiteLabelConfig(full)).toEqual(full)
    })

    it('preserves a valid partial config (some fields set, rest null)', () => {
      // A partial *valid* input means: every key present, some with values and
      // the rest null. This is the realistic "only logo configured" shape.
      const partial = {
        ...EMPTY_FORM,
        logoUrl: 'https://cdn.example.com/logo.svg',
      }

      expect(normalizeWhiteLabelConfig(partial)).toEqual(partial)
    })

    it('strips unknown fields without failing', () => {
      // Backend payloads may carry `updatedAt` / `message` etc. normalize must
      // drop them silently rather than reject the whole object.
      const withExtra = {
        ...makeFullWhiteLabelConfig(),
        updatedAt: '2026-07-08T00:00:00Z',
        message: 'should-be-removed',
      }

      const result = normalizeWhiteLabelConfig(withExtra)

      expect(result).toEqual(makeFullWhiteLabelConfig())
      expect(result).not.toHaveProperty('updatedAt')
      expect(result).not.toHaveProperty('message')
    })

    it('preserves empty strings (no normalization at this layer)', () => {
      // normalize is *not* the empty-string->null step. The form can hold empty
      // inputs after load; conversion happens in toUpdateWhiteLabelConfigRequest.
      const result = normalizeWhiteLabelConfig({ ...EMPTY_FORM, logoUrl: '' })

      expect(result.logoUrl).toBe('')
    })
  })

  describe('does not throw on malformed stored JSON strings', () => {
    // Some realm_config rows store configValue as a JSON string; if a caller
    // passes the raw (un-parsed) string into normalize, it must degrade
    // gracefully to the empty form rather than throw.
    it('does not throw for an unparsable JSON string', () => {
      expect(() => normalizeWhiteLabelConfig('{ this is not json')).not.toThrow()
    })
  })
})

describe('toUpdateWhiteLabelConfigRequest', () => {
  describe('converts empty / whitespace-only strings to null', () => {
    // The wire contract sends `null` for "no value"; the form may temporarily
    // hold empty or whitespace strings. The request builder is the single
    // place where the empty-string -> null collapse happens.
    it.each([
      ['empty string', ''],
      ['single space', ' '],
      ['multiple spaces', '   '],
      ['tabs and spaces', '\t  \n'],
    ])('normalizes logoUrl=%j to null', (_label, value) => {
      const result = toUpdateWhiteLabelConfigRequest(makeFullWhiteLabelConfig({ logoUrl: value }))

      expect(result.logoUrl).toBeNull()
    })

    it('normalizes every string field from empty strings', () => {
      const form = {
        logoUrl: '',
        accentColor: '',
        background: null,
        footerText: '',
        loginTitle: '',
        loginSubtitle: '',
        registerTitle: '',
        registerSubtitle: '',
      }

      expect(toUpdateWhiteLabelConfigRequest(form)).toEqual({
        logoUrl: null,
        accentColor: null,
        background: null,
        footerText: null,
        loginTitle: null,
        loginSubtitle: null,
        registerTitle: null,
        registerSubtitle: null,
      })
    })
  })

  describe('preserves non-empty values and trims whitespace', () => {
    it('keeps a valid logoUrl and trims surrounding whitespace', () => {
      const result = toUpdateWhiteLabelConfigRequest(
        makeFullWhiteLabelConfig({ logoUrl: '  https://cdn.example.com/logo.svg  ' })
      )

      expect(result.logoUrl).toBe('https://cdn.example.com/logo.svg')
    })

    it('preserves every populated string field', () => {
      const result = toUpdateWhiteLabelConfigRequest(makeFullWhiteLabelConfig())

      expect(result).toEqual({
        logoUrl: 'https://cdn.example.com/logo.svg',
        accentColor: '#2563eb',
        background: { type: 'gradient', value: 'linear-gradient(135deg, #1e3a8a, #2563eb)' },
        footerText: 'Example Inc.',
        loginTitle: 'Sign in to Example',
        loginSubtitle: 'Use your Example account',
        registerTitle: 'Create your Example account',
        registerSubtitle: 'Start with Example',
      })
    })
  })

  describe('background normalization', () => {
    it('collapses a background with an empty value to null', () => {
      // A background object whose value is empty/whitespace is meaningless; the
      // builder drops the whole object to null so the backend stores nothing.
      const result = toUpdateWhiteLabelConfigRequest(
        makeFullWhiteLabelConfig({
          background: { type: 'image', value: '   ' },
        })
      )

      expect(result.background).toBeNull()
    })

    it('trims a populated background value but keeps type', () => {
      const result = toUpdateWhiteLabelConfigRequest(
        makeFullWhiteLabelConfig({
          background: { type: 'gradient', value: '  linear-gradient(#000, #111)  ' },
        })
      )

      expect(result.background).toEqual({ type: 'gradient', value: 'linear-gradient(#000, #111)' })
    })

    it('passes a null background through as null', () => {
      const result = toUpdateWhiteLabelConfigRequest(makeFullWhiteLabelConfig({ background: null }))

      expect(result.background).toBeNull()
    })
  })

  describe('passes all-null form through unchanged', () => {
    it('produces an all-null request for an unconfigured realm', () => {
      const result = toUpdateWhiteLabelConfigRequest(emptyWhiteLabelConfig())

      expect(result).toEqual(EMPTY_FORM)
    })
  })

  describe('drops unknown fields (matches UpdateWhiteLabelConfigRequest shape)', () => {
    // The returned object must match the generated wire type exactly. Even if a
    // caller sneaks extra keys onto the form, the builder only ever emits the 8
    // declared fields. (z.object strips extras on parse; this asserts the
    // builder's own output never invents new keys.)
    it('never emits more than the 8 wire fields', () => {
      const result = toUpdateWhiteLabelConfigRequest(
        makeFullWhiteLabelConfig({
          // TS would block this at compile time, but runtime payloads from a
          // loosely-typed source could carry extras.
          ...({ extraField: 'leak' } as Record<string, unknown>),
        })
      )

      expect(Object.keys(result).sort()).toEqual(
        [
          'logoUrl',
          'accentColor',
          'background',
          'footerText',
          'loginTitle',
          'loginSubtitle',
          'registerTitle',
          'registerSubtitle',
        ].sort()
      )
    })
  })
})
