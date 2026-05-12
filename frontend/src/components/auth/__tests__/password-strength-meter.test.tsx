import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PasswordStrengthMeter } from '../password-strength-meter'
import type { PasswordConfig } from '@/lib/password-strength'

describe('PasswordStrengthMeter', () => {
  const defaultConfig: PasswordConfig = {
    minLength: 8,
    requireUppercase: true,
    requireLowercase: true,
    requireNumber: true,
    requireSpecialChar: true,
  }

  describe('rendering', () => {
    it('GIVEN weak password WHEN rendering THEN displays Weak label and empty bar', async () => {
      const screen = render(<PasswordStrengthMeter password="" config={defaultConfig} />)
      expect(screen.getByText('Weak')).toBeInTheDocument()
      const bar = screen.container.querySelector('[style*="width: 0%"]')
      expect(bar).toBeInTheDocument()
    })

    it('GIVEN strong password WHEN rendering THEN displays Strong label and full bar', async () => {
      const screen = render(
        <PasswordStrengthMeter password="Password123!" config={defaultConfig} />
      )
      expect(screen.getByText('Strong')).toBeInTheDocument()
      const bar = screen.container.querySelector('.bg-green-500')
      expect(bar).toBeInTheDocument()
    })
  })

  describe('suggestions', () => {
    it('GIVEN password with missing requirements WHEN rendering THEN displays all relevant suggestions', async () => {
      const screen = render(<PasswordStrengthMeter password="abc" config={defaultConfig} />)
      expect(screen.getByText(/must be at least 8 characters/)).toBeInTheDocument()
      expect(screen.getByText(/must contain uppercase letters/)).toBeInTheDocument()
      expect(screen.getByText(/must contain numbers/)).toBeInTheDocument()
      expect(screen.getByText(/must contain special characters/)).toBeInTheDocument()
      const list = screen.container.querySelector('ul')
      expect(list).toBeInTheDocument()
    })

    it('GIVEN password missing uppercase WHEN rendering THEN displays uppercase suggestion', async () => {
      const screen = render(
        <PasswordStrengthMeter password="password123!" config={defaultConfig} />
      )
      expect(screen.getByText(/must contain uppercase letters/)).toBeInTheDocument()
    })

    it('GIVEN password missing lowercase WHEN rendering THEN displays lowercase suggestion', async () => {
      const screen = render(
        <PasswordStrengthMeter password="PASSWORD123!" config={defaultConfig} />
      )
      expect(screen.getByText(/must contain lowercase letters/)).toBeInTheDocument()
    })

    it('GIVEN strong password WHEN rendering THEN hides suggestions', async () => {
      const screen = render(
        <PasswordStrengthMeter password="Password123!" config={defaultConfig} />
      )
      const list = screen.container.querySelector('ul')
      expect(list).not.toBeInTheDocument()
    })
  })

  describe('config variations', () => {
    it('GIVEN minimal config WHEN rendering THEN works correctly', async () => {
      const minimalConfig: PasswordConfig = {
        minLength: 1,
        requireUppercase: false,
        requireLowercase: false,
        requireNumber: false,
        requireSpecialChar: false,
      }
      const screen = render(<PasswordStrengthMeter password="a" config={minimalConfig} />)
      expect(screen.getByText(/Weak|Fair|Good|Strong/)).toBeInTheDocument()
    })

    it('GIVEN unicode password WHEN rendering THEN calculates strength correctly', async () => {
      const screen = render(
        <PasswordStrengthMeter password="Password123!é" config={defaultConfig} />
      )
      expect(screen.getByText('Strong')).toBeInTheDocument()
    })
  })
})
