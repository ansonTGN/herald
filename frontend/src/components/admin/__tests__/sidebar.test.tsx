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
  useQuery: () => ({ data: { id: 'admin', name: 'Admin Realm' } }),
}))

vi.mock('@/data/query-options', () => ({
  realmQueryOptions: () => ({ queryKey: ['realm', 'admin'] }),
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

  it('highlights billing plans on the billing page (under Products & Pricing)', async () => {
    currentPath = '/admin/manage/billing?page=0&pageSize=20&status=all'
    const user = userEvent.setup()
    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-products-&-pricing'))

    const billingPlansLink = screen.getByTestId('sidebar-menu-billing-plans')
    const productsLink = screen.getByTestId('sidebar-menu-products')

    expect(billingPlansLink).toHaveClass('font-semibold')
    expect(productsLink).not.toHaveClass('font-semibold')
  })

  it('highlights invoices on the invoices page (under Billing)', async () => {
    currentPath = '/admin/manage/billing/invoices'
    const user = userEvent.setup()
    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-billing'))

    const invoicesLink = screen.getByTestId('sidebar-menu-invoices')
    const providersLink = screen.getByTestId('sidebar-menu-payment-providers')

    expect(invoicesLink).toHaveClass('font-semibold')
    expect(providersLink).not.toHaveClass('font-semibold')
  })

  it('highlights only payment providers on the payment providers page (under Billing)', async () => {
    currentPath = '/admin/manage/billing/payment-providers'
    const user = userEvent.setup()
    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-billing'))

    const providersLink = screen.getByTestId('sidebar-menu-payment-providers')
    const invoicesLink = screen.getByTestId('sidebar-menu-invoices')

    expect(providersLink).toHaveClass('font-semibold')
    expect(invoicesLink).not.toHaveClass('font-semibold')
  })

  it('keeps sidebar navigation in its own scroll container when group expands', async () => {
    currentPath = '/admin/manage/billing?page=0&pageSize=20&status=all'
    const user = userEvent.setup()
    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-products-&-pricing'))

    const sidebar = screen.getByTestId('admin-sidebar')
    const nav = screen.getByTestId('sidebar-nav')

    expect(sidebar).toHaveClass('h-full', 'min-h-0', 'flex', 'flex-col')
    expect(nav).toHaveClass('min-h-0', 'flex-1', 'overflow-y-auto')
  })
})
