import { describe, it, expect } from 'vitest'
import { createCreditBucketSchema, updateCreditBucketSchema } from '../credit-bucket-forms'

/**
 * Factory for valid Create Credit Bucket input.
 * `enabled` / `receivesRegistrationCredits` are required booleans in the schema
 * (no `.default()`), so the factory must supply them explicitly.
 */
function validCreateBucketInput(overrides: Record<string, unknown> = {}) {
  return {
    bucketKey: 'main-bucket',
    name: 'Main Bucket',
    enabled: true,
    receivesRegistrationCredits: false,
    clientAppIds: ['app-1'],
    ...overrides,
  }
}

/** Factory for valid Update Credit Bucket input (no `bucketKey`). */
function validUpdateBucketInput(overrides: Record<string, unknown> = {}) {
  return {
    name: 'Main Bucket',
    enabled: true,
    receivesRegistrationCredits: false,
    clientAppIds: ['app-1'],
    ...overrides,
  }
}

describe('createCreditBucketSchema', () => {
  describe('bucketKey accepts valid keys', () => {
    it.each([
      ['single character', 'a'],
      ['lowercase with hyphen', 'a-b'],
      ['mixed alphanumeric', 'abc123'],
      ['max length 64 chars', 'a'.repeat(64)],
    ])('accepts %s: %s', (_label, key) => {
      const result = createCreditBucketSchema.safeParse(validCreateBucketInput({ bucketKey: key }))
      expect(result.success).toBe(true)
    })
  })

  describe('bucketKey rejects invalid keys', () => {
    it.each([
      ['empty string', ''],
      ['uppercase letter', 'A'],
      ['underscore', 'a_b'],
      ['dot', 'a.b'],
      ['over 64 chars', 'a'.repeat(65)],
      ['contains space', 'a b'],
      ['chinese characters', '中文'],
    ])('rejects %s: %s', (_label, key) => {
      const result = createCreditBucketSchema.safeParse(validCreateBucketInput({ bucketKey: key }))
      expect(result.success).toBe(false)
    })
  })

  describe('clientAppIds fails loud when coverage set is empty', () => {
    it.each([
      ['empty array', []],
      ['undefined field', undefined],
    ])('rejects %s', (_label, clientAppIds) => {
      const input = validCreateBucketInput()
      delete (input as { clientAppIds?: unknown }).clientAppIds
      if (clientAppIds !== undefined) {
        input.clientAppIds = clientAppIds
      }
      const result = createCreditBucketSchema.safeParse(input)
      expect(result.success).toBe(false)
    })

    it('rejects array containing empty string id', () => {
      const result = createCreditBucketSchema.safeParse(
        validCreateBucketInput({ clientAppIds: [''] })
      )
      expect(result.success).toBe(false)
    })
  })

  describe('required fields', () => {
    it('rejects when name is missing', () => {
      const input = validCreateBucketInput()
      delete (input as { name?: unknown }).name
      const result = createCreditBucketSchema.safeParse(input)
      expect(result.success).toBe(false)
    })

    it('rejects when name is empty string', () => {
      const result = createCreditBucketSchema.safeParse(validCreateBucketInput({ name: '' }))
      expect(result.success).toBe(false)
    })
  })

  describe('optional fields can be omitted without error', () => {
    it('passes with only required fields (no description/displayOrder/entitlementMappingIds)', () => {
      const result = createCreditBucketSchema.safeParse(validCreateBucketInput())
      expect(result.success).toBe(true)
    })

    it('passes with entitlementMappingIds provided', () => {
      const result = createCreditBucketSchema.safeParse(
        validCreateBucketInput({ entitlementMappingIds: ['em-1', 'em-2'] })
      )
      expect(result.success).toBe(true)
    })
  })

  describe('valid full input parses with correct output', () => {
    it('keeps provided optional values and defaults absent ones to undefined', () => {
      const result = createCreditBucketSchema.safeParse(
        validCreateBucketInput({
          description: 'A bucket',
          displayOrder: 3,
          entitlementMappingIds: ['em-1'],
        })
      )
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.bucketKey).toBe('main-bucket')
        expect(result.data.name).toBe('Main Bucket')
        expect(result.data.enabled).toBe(true)
        expect(result.data.receivesRegistrationCredits).toBe(false)
        expect(result.data.clientAppIds).toEqual(['app-1'])
        expect(result.data.description).toBe('A bucket')
        expect(result.data.displayOrder).toBe(3)
        expect(result.data.entitlementMappingIds).toEqual(['em-1'])
      }
    })
  })
})

describe('updateCreditBucketSchema', () => {
  describe('bucketKey is not part of the update contract', () => {
    it('passes when bucketKey is omitted', () => {
      const result = updateCreditBucketSchema.safeParse(validUpdateBucketInput())
      expect(result.success).toBe(true)
    })

    // bucketKey is immutable identity. zod strips unknown keys
    // by default, so submitting a bucketKey on update does NOT fail — it is
    // silently dropped. Assert it never lands in the parsed output so a future
    // schema change (e.g. .strict()) cannot silently start mutating identity.
    it('strips bucketKey from the parsed output (immutable identity)', () => {
      const result = updateCreditBucketSchema.safeParse(
        validUpdateBucketInput({ bucketKey: 'should-be-ignored' })
      )
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data).not.toHaveProperty('bucketKey')
      }
    })
  })

  describe('clientAppIds fails loud when coverage set is empty', () => {
    it('rejects empty array', () => {
      const result = updateCreditBucketSchema.safeParse(
        validUpdateBucketInput({ clientAppIds: [] })
      )
      expect(result.success).toBe(false)
    })
  })

  describe('required fields', () => {
    it('rejects when name is missing', () => {
      const input = validUpdateBucketInput()
      delete (input as { name?: unknown }).name
      const result = updateCreditBucketSchema.safeParse(input)
      expect(result.success).toBe(false)
    })
  })

  describe('valid full input parses', () => {
    it('passes with all required fields', () => {
      const result = updateCreditBucketSchema.safeParse(validUpdateBucketInput())
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.name).toBe('Main Bucket')
        expect(result.data.clientAppIds).toEqual(['app-1'])
      }
    })
  })
})
