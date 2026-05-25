import { describe, it, expect } from 'vitest'
import {
  pointsPackageFormSchema,
  paymentProviderMappingSchema,
  getPointsPackageDefaults,
  getPaymentProviderMappingDefaults,
  updatePointsPackageFormSchema,
  getUpdatePointsPackageDefaults,
  calculateDiscountPercent,
} from '../points-package-forms'

describe('Points Package Forms', () => {
  describe('pointsPackageFormSchema', () => {
    it('should validate correct data', () => {
      const data = {
        name: 'basic-package',
        title: 'Basic Package',
        description: 'A basic points package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
        packageType: 'standard',
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
    })

    it('should reject invalid package name', () => {
      const data = {
        name: 'Invalid_Name!',
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject package name less than 3 characters', () => {
      const data = {
        name: 'ab',
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject title less than 1 character', () => {
      const data = {
        name: 'test-package',
        title: '',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject negative points', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: -10,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject price less than 0.01', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 100,
        price: 0,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject invalid currency format', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'US', // Should be exactly 3 characters
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject negative sort order', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: -1,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should default packageType to standard when omitted', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.packageType).toBe('standard')
      }
    })
  })

  describe('paymentProviderMappingSchema', () => {
    it('should validate correct data', () => {
      const data = {
        paymentProvider: 'stripe',
        enabled: true,
        externalProductId: 'prod_12345',
      }

      const result = paymentProviderMappingSchema.safeParse(data)
      expect(result.success).toBe(true)
    })

    it('should reject invalid provider', () => {
      const data = {
        paymentProvider: 'invalid_provider',
        enabled: true,
      }

      const result = paymentProviderMappingSchema.safeParse(data)
      expect(result.success).toBe(false)
    })
  })

  describe('getPointsPackageDefaults', () => {
    it('should return correct defaults', () => {
      const defaults = getPointsPackageDefaults()

      expect(defaults).toEqual({
        name: '',
        title: '',
        description: '',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
        packageType: 'standard',
        originalPrice: undefined,
        promoStartTime: '',
        promoEndTime: '',
      })
    })

    it('should override defaults with provided values', () => {
      const overrides = {
        name: 'custom-package',
        points: 500,
      }

      const defaults = getPointsPackageDefaults(overrides)

      expect(defaults.name).toBe('custom-package')
      expect(defaults.points).toBe(500)
      expect(defaults.price).toBe(9.99) // Default value
    })

    it('should convert originalPrice from API cents to display price', () => {
      const defaults = getPointsPackageDefaults({
        originalPrice: 1999, // 1999 cents
        currency: 'USD',
      })

      expect(defaults.originalPrice).toBe(19.99)
    })

    it('should handle promo fields from input', () => {
      const defaults = getPointsPackageDefaults({
        packageType: 'promotional',
        originalPrice: 2999, // cents
        promoStartTime: '2025-01-01T00:00:00Z',
        promoEndTime: '2025-12-31T23:59:59Z',
        currency: 'USD',
      })

      expect(defaults.packageType).toBe('promotional')
      expect(defaults.originalPrice).toBe(29.99)
      expect(defaults.promoStartTime).toBe('2025-01-01T00:00:00Z')
      expect(defaults.promoEndTime).toBe('2025-12-31T23:59:59Z')
    })
  })

  describe('getPaymentProviderMappingDefaults', () => {
    it('should return correct defaults', () => {
      const defaults = getPaymentProviderMappingDefaults()

      expect(defaults).toEqual({
        paymentProvider: 'stripe',
        enabled: true,
        externalProductId: '',
      })
    })

    it('should override defaults with provided values', () => {
      const overrides = {
        paymentProvider: 'wechat' as const,
        enabled: false,
      }

      const defaults = getPaymentProviderMappingDefaults(overrides)

      expect(defaults.paymentProvider).toBe('wechat')
      expect(defaults.enabled).toBe(false)
      expect(defaults.externalProductId).toBe('') // Default value
    })
  })

  describe('updatePointsPackageFormSchema', () => {
    it('should accept valid update data', () => {
      const data = {
        title: 'Updated Title',
        price: 19.99,
        enabled: false,
      }

      const result = updatePointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
    })

    it('should reject invalid title in update', () => {
      const data = {
        title: '',
      }

      const result = updatePointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject negative price in update', () => {
      const data = {
        price: -10,
      }

      const result = updatePointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should allow partial updates', () => {
      const data = {
        description: 'New description',
      }

      const result = updatePointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
    })

    it('should accept promo fields in update', () => {
      const data = {
        packageType: 'promotional',
        price: 9.99,
        originalPrice: 19.99,
        promoStartTime: '2025-06-01T00:00',
        promoEndTime: '2025-12-31T23:59',
      }

      const result = updatePointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
    })
  })

  describe('pointsPackageFormSchema edge cases', () => {
    it('should reject package name exceeding max length', () => {
      const data = {
        name: 'a'.repeat(51), // Exceeds 50 characters
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject title exceeding max length', () => {
      const data = {
        name: 'test-package',
        title: 'a'.repeat(101), // Exceeds 100 characters
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject description exceeding max length', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        description: 'a'.repeat(501), // Exceeds 500 characters
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject price exceeding max value', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 100,
        price: 10000000, // Exceeds 9999999
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject non-integer points', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 100.5, // Not an integer
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject non-integer sort order', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 1.5, // Not an integer
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject lowercase currency code', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'usd', // Should be uppercase
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should accept valid boundary values', () => {
      const data = {
        name: 'abc', // Min length 3
        title: 'A', // Min length 1
        points: 1, // Min value
        price: 0.01, // Min value
        currency: 'USD',
        sortOrder: 0, // Min value
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
    })

    it('should reject zero points', () => {
      const data = {
        name: 'test-package',
        title: 'Test Package',
        points: 0, // Must be at least 1
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject package name with invalid characters', () => {
      const data = {
        name: 'test.package', // Invalid character '.'
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should reject package name with uppercase letters', () => {
      const data = {
        name: 'TestPackage', // Uppercase not allowed
        title: 'Test Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
    })
  })

  describe('promo cross-field validation', () => {
    it('should accept valid promotional package', () => {
      const data = {
        name: 'promo-pkg',
        title: 'Promo Package',
        points: 100,
        price: 4.99,
        currency: 'USD',
        packageType: 'promotional',
        originalPrice: 9.99,
        promoStartTime: '2025-06-01T00:00',
        promoEndTime: '2025-12-31T23:59',
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
    })

    it('should reject promotional package without originalPrice', () => {
      const data = {
        name: 'promo-pkg',
        title: 'Promo Package',
        points: 100,
        price: 4.99,
        currency: 'USD',
        packageType: 'promotional',
        promoEndTime: '2025-12-31T23:59',
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
      if (!result.success) {
        const issues = result.error.issues
        expect(issues.some((i) => i.path.includes('originalPrice'))).toBe(true)
      }
    })

    it('should reject promotional package where originalPrice <= price', () => {
      const data = {
        name: 'promo-pkg',
        title: 'Promo Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        packageType: 'promotional',
        originalPrice: 9.99, // Same as price, not greater
        promoEndTime: '2025-12-31T23:59',
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
      if (!result.success) {
        const issues = result.error.issues
        expect(issues.some((i) => i.path.includes('originalPrice'))).toBe(true)
      }
    })

    it('should reject promotional package without promoEndTime', () => {
      const data = {
        name: 'promo-pkg',
        title: 'Promo Package',
        points: 100,
        price: 4.99,
        currency: 'USD',
        packageType: 'promotional',
        originalPrice: 9.99,
        promoStartTime: '2025-06-01T00:00',
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
      if (!result.success) {
        const issues = result.error.issues
        expect(issues.some((i) => i.path.includes('promoEndTime'))).toBe(true)
      }
    })

    it('should reject when promoEndTime is before promoStartTime', () => {
      const data = {
        name: 'promo-pkg',
        title: 'Promo Package',
        points: 100,
        price: 4.99,
        currency: 'USD',
        packageType: 'promotional',
        originalPrice: 9.99,
        promoStartTime: '2025-12-31T23:59',
        promoEndTime: '2025-01-01T00:00', // Before start
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(false)
      if (!result.success) {
        const issues = result.error.issues
        expect(issues.some((i) => i.path.includes('promoEndTime'))).toBe(true)
      }
    })

    it('should accept standard package without promo fields', () => {
      const data = {
        name: 'standard-pkg',
        title: 'Standard Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        packageType: 'standard',
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
    })

    it('should accept standard package even with empty promo fields', () => {
      const data = {
        name: 'standard-pkg',
        title: 'Standard Package',
        points: 100,
        price: 9.99,
        currency: 'USD',
        packageType: 'standard',
        originalPrice: undefined,
        promoStartTime: '',
        promoEndTime: '',
      }

      const result = pointsPackageFormSchema.safeParse(data)
      expect(result.success).toBe(true)
    })
  })

  describe('paymentProviderMappingSchema edge cases', () => {
    it('should reject externalProductId exceeding max length', () => {
      const data = {
        paymentProvider: 'stripe' as const,
        enabled: true,
        externalProductId: 'a'.repeat(256), // Exceeds 255 characters
      }

      const result = paymentProviderMappingSchema.safeParse(data)
      expect(result.success).toBe(false)
    })

    it('should default enabled to true', () => {
      const data = {
        paymentProvider: 'stripe' as const,
      }

      const result = paymentProviderMappingSchema.safeParse(data)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.enabled).toBe(true)
      }
    })
  })

  describe('getUpdatePointsPackageDefaults', () => {
    it('should return correct defaults', () => {
      const pkg = {
        name: 'test-package',
        title: 'Test Package',
        description: 'Test Description',
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
      }

      const defaults = getUpdatePointsPackageDefaults(pkg)

      expect(defaults).toEqual({
        name: 'test-package',
        title: 'Test Package',
        description: 'Test Description',
        price: 9.99,
        currency: 'USD',
        sortOrder: 0,
        enabled: true,
        packageType: 'standard',
        originalPrice: undefined,
        promoStartTime: '',
        promoEndTime: '',
      })
    })

    it('should handle null description', () => {
      const pkg = {
        name: 'test-package',
        description: null,
      }

      const defaults = getUpdatePointsPackageDefaults(pkg)

      expect(defaults.description).toBe('')
    })

    it('should provide defaults for missing fields', () => {
      const pkg = {
        name: 'test-package',
      }

      const defaults = getUpdatePointsPackageDefaults(pkg)

      expect(defaults.title).toBe('')
      expect(defaults.price).toBe(9.99)
      expect(defaults.currency).toBe('USD')
      expect(defaults.sortOrder).toBe(0)
      expect(defaults.enabled).toBe(true)
    })

    it('should convert originalPrice from API cents for update', () => {
      const pkg = {
        name: 'promo-pkg',
        packageType: 'promotional',
        originalPrice: 2999, // cents
        currency: 'USD',
        promoStartTime: '2025-06-01T00:00:00Z',
        promoEndTime: '2025-12-31T23:59:59Z',
      }

      const defaults = getUpdatePointsPackageDefaults(pkg)

      expect(defaults.packageType).toBe('promotional')
      expect(defaults.originalPrice).toBe(29.99)
      expect(defaults.promoStartTime).toBe('2025-06-01T00:00:00Z')
      expect(defaults.promoEndTime).toBe('2025-12-31T23:59:59Z')
    })
  })

  describe('calculateDiscountPercent', () => {
    it('should calculate 50% discount', () => {
      expect(calculateDiscountPercent(5, 10)).toBe(50)
    })

    it('should calculate 0% discount when prices are equal', () => {
      expect(calculateDiscountPercent(10, 10)).toBe(0)
    })

    it('should calculate 25% discount', () => {
      expect(calculateDiscountPercent(7.5, 10)).toBe(25)
    })

    it('should round to nearest integer', () => {
      // 1 - 3.33 / 9.99 = 0.667... rounds to 67
      expect(calculateDiscountPercent(3.33, 9.99)).toBe(67)
    })

    it('should handle 99% discount', () => {
      expect(calculateDiscountPercent(0.01, 1)).toBe(99)
    })
  })
})
