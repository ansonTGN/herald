/**
 * @vitest-environment jsdom
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Sidebar } from '../sidebar'
import { useAuthStore } from '@/stores/auth-store'

let currentPath = '/admin/manage/billing?page=0&pageSize=20&status=all'

vi.mock('@tanstack/react-query', () => ({
  useQuery: () => ({ data: { id: 'admin', name: 'Admin' } }),
}))

vi.mock('@/data/query-options', () => ({
  realmQueryOptions: () => ({ queryKey: ['realm', 'admin'] }),
  featureAvailabilityQueryOptions: () => ({ queryKey: ['feature-availability', 'admin'] }),
}))

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    to,
    className,
    activeProps,
    activeOptions,
    children,
    ...props
  }: {
    to: string
    className?: string
    activeProps?: { className?: string }
    activeOptions?: { exact?: boolean }
    children: React.ReactNode
  }) => {
    let isActive: boolean
    if (activeOptions?.exact) {
      const pathWithoutQuery = currentPath.split('?')[0]
      isActive = pathWithoutQuery === to
    } else {
      isActive =
        currentPath === to || currentPath.startsWith(`${to}/`) || currentPath.startsWith(`${to}?`)
    }

    const resolvedClassName = [className, isActive ? activeProps?.className : undefined]
      .filter(Boolean)
      .join(' ')

    return (
      <a href={to} className={resolvedClassName} {...props}>
        {children}
      </a>
    )
  },
}))

describe('Sidebar navigation', () => {
  beforeEach(() => {
    useAuthStore.setState({
      isAuthenticated: true,
      isLoading: false,
      realmId: 'admin',
      user: null,
      permissions: ['billing.view'],
      roles: [],
    })
  })

  afterEach(() => {
    cleanup()
    useAuthStore.getState().reset()
  })

  it('highlights subscription plans on the billing page', async () => {
    currentPath = '/admin/manage/billing?page=0&pageSize=20&status=all'
    const user = userEvent.setup()
    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-products-&-payments'))

    const subscriptionPlansLink = screen.getByTestId('sidebar-menu-subscription-plans')
    const paymentProvidersLink = screen.getByTestId('sidebar-menu-payment-providers')

    expect(subscriptionPlansLink).toHaveClass('font-semibold')
    expect(paymentProvidersLink).not.toHaveClass('font-semibold')
  })

  it('highlights invoices on the invoices page (under Transactions)', async () => {
    currentPath = '/admin/manage/billing/invoices'
    const user = userEvent.setup()
    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-transactions'))

    const invoicesLink = screen.getByTestId('sidebar-menu-invoices')
    const subscriptionHistoryLink = screen.getByTestId('sidebar-menu-subscription-history')

    expect(invoicesLink).toHaveClass('font-semibold')
    expect(subscriptionHistoryLink).not.toHaveClass('font-semibold')
  })

  it('highlights only payment providers on the payment providers page (under Products & Payments)', async () => {
    currentPath = '/admin/manage/billing/payment-providers'
    const user = userEvent.setup()
    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-products-&-payments'))

    const providersLink = screen.getByTestId('sidebar-menu-payment-providers')
    const productsLink = screen.getByTestId('sidebar-menu-products')

    expect(providersLink).toHaveClass('font-semibold')
    expect(productsLink).not.toHaveClass('font-semibold')
  })

  it('keeps sidebar navigation in its own scroll container when group expands', async () => {
    currentPath = '/admin/manage/billing?page=0&pageSize=20&status=all'
    const user = userEvent.setup()
    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-products-&-payments'))

    const sidebar = screen.getByTestId('admin-sidebar')
    const nav = screen.getByTestId('sidebar-nav')

    expect(sidebar).toHaveClass('h-full', 'min-h-0', 'flex', 'flex-col')
    expect(nav).toHaveClass('min-h-0', 'flex-1', 'overflow-y-auto')
  })
})
