import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import type { ReactNode } from 'react'
import { LegalLinks } from '../legal-links'

vi.mock('@tanstack/react-router', async () => {
  const actual =
    await vi.importActual<typeof import('@tanstack/react-router')>('@tanstack/react-router')
  return {
    ...actual,
    Link: ({
      to,
      params,
      children,
      ...props
    }: {
      to: string
      params?: Record<string, string>
      children?: ReactNode
    }) => {
      let href = to as string
      if (params) {
        Object.entries(params).forEach(([key, value]) => {
          href = href.replace(new RegExp(`\\$\\{${key}\\}|\\$${key}`, 'g'), value)
        })
      }
      return (
        <a href={href} {...props}>
          {children}
        </a>
      )
    },
  }
})

describe('LegalLinks', () => {
  const realmId = 'test-realm'

  it('renders a card with Terms of Service and Privacy Policy links', () => {
    render(<LegalLinks realmId={realmId} />)

    expect(screen.getByTestId('legal-links-card')).toBeInTheDocument()
    expect(screen.getByTestId('legal-links-title')).toBeInTheDocument()
    expect(screen.getByTestId('legal-links-content')).toBeInTheDocument()

    const termsLink = screen.getByTestId('terms-of-service-link')
    const privacyLink = screen.getByTestId('privacy-policy-link')

    expect(termsLink).toHaveTextContent('Terms of Service')
    expect(termsLink).toHaveAttribute('href', `/${realmId}/legal/terms_of_service`)

    expect(privacyLink).toHaveTextContent('Privacy Policy')
    expect(privacyLink).toHaveAttribute('href', `/${realmId}/legal/privacy_policy`)
  })
})
