import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { AgreementLinks } from '../AgreementLinks'

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
      children?: React.ReactNode
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

describe('AgreementLinks', () => {
  const realmId = 'test-realm'

  it('GIVEN a realmId WHEN rendering THEN shows ToS and Privacy Policy links with correct hrefs', async () => {
    render(<AgreementLinks realmId={realmId} beforeText="I agree to " />)

    const termsLink = screen.getByTestId('terms-of-service-link')
    const privacyLink = screen.getByTestId('privacy-policy-link')

    expect(termsLink).toHaveTextContent('Terms of Service')
    expect(termsLink).toHaveAttribute('href', `/${realmId}/legal/terms_of_service`)
    expect(privacyLink).toHaveTextContent('Privacy Policy')
    expect(privacyLink).toHaveAttribute('href', `/${realmId}/legal/privacy_policy`)
  })

  it('GIVEN custom classes WHEN rendering THEN applies them to the container', async () => {
    const { container } = render(<AgreementLinks realmId={realmId} className="custom-class" />)

    expect(container.firstChild).toHaveClass('custom-class')
  })
})
