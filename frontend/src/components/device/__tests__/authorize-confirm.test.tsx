import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { AuthorizeConfirm } from '../authorize-confirm'

describe('AuthorizeConfirm', () => {
  it('renders client app name', () => {
    render(
      <AuthorizeConfirm
        clientAppName="My Test App"
        onConfirm={vi.fn()}
        isLoading={false}
      />
    )

    expect(screen.getByText('My Test App')).toBeInTheDocument()
  })

  it('renders client app icon when provided', () => {
    render(
      <AuthorizeConfirm
        clientAppName="My Test App"
        clientAppIconUrl="https://example.com/icon.png"
        onConfirm={vi.fn()}
        isLoading={false}
      />
    )

    const icon = screen.getByRole('img', { name: /my test app icon/i })
    expect(icon).toBeInTheDocument()
    expect(icon).toHaveAttribute('src', 'https://example.com/icon.png')
  })

  it('calls onConfirm(true) when Authorize button is clicked', async () => {
    const onConfirm = vi.fn()
    render(
      <AuthorizeConfirm
        clientAppName="My Test App"
        onConfirm={onConfirm}
        isLoading={false}
      />
    )

    await userEvent.click(screen.getByTestId('device-authorize-button'))
    expect(onConfirm).toHaveBeenCalledWith(true)
  })

  it('calls onConfirm(false) when Deny button is clicked', async () => {
    const onConfirm = vi.fn()
    render(
      <AuthorizeConfirm
        clientAppName="My Test App"
        onConfirm={onConfirm}
        isLoading={false}
      />
    )

    await userEvent.click(screen.getByTestId('device-deny-button'))
    expect(onConfirm).toHaveBeenCalledWith(false)
  })

  it('disables both buttons when isLoading is true', () => {
    render(
      <AuthorizeConfirm
        clientAppName="My Test App"
        onConfirm={vi.fn()}
        isLoading={true}
      />
    )

    expect(screen.getByTestId('device-authorize-button')).toBeDisabled()
    expect(screen.getByTestId('device-deny-button')).toBeDisabled()
  })

  it('enables both buttons when isLoading is false', () => {
    render(
      <AuthorizeConfirm
        clientAppName="My Test App"
        onConfirm={vi.fn()}
        isLoading={false}
      />
    )

    expect(screen.getByTestId('device-authorize-button')).toBeEnabled()
    expect(screen.getByTestId('device-deny-button')).toBeEnabled()
  })
})
