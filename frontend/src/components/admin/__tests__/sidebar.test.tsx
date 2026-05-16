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

describe('Sidebar billing navigation', () => {
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

  async function renderSidebar(pathname: string) {
    currentPath = pathname
    const user = userEvent.setup()

    render(<Sidebar />)

    await user.click(screen.getByTestId('sidebar-menu-billing'))
  }

  it('highlights billing plans on the billing page', async () => {
    await renderSidebar('/admin/manage/billing?page=0&pageSize=20&status=all')

    const billingPlansLink = screen.getByTestId('sidebar-menu-billing-plans')
    const invoicesLink = screen.getByTestId('sidebar-menu-invoices')

    expect(billingPlansLink).toHaveClass('font-semibold')
    expect(invoicesLink).not.toHaveClass('font-semibold')
  })

  it('highlights invoices on the invoices page', async () => {
    await renderSidebar('/admin/manage/billing/invoices')

    const billingPlansLink = screen.getByTestId('sidebar-menu-billing-plans')
    const invoicesLink = screen.getByTestId('sidebar-menu-invoices')

    expect(billingPlansLink).not.toHaveClass('font-semibold')
    expect(invoicesLink).toHaveClass('font-semibold')
  })

  it('highlights only payment providers on the payment providers page', async () => {
    await renderSidebar('/admin/manage/billing/payment-providers')

    const billingPlansLink = screen.getByTestId('sidebar-menu-billing-plans')
    const providersLink = screen.getByTestId('sidebar-menu-payment-providers')

    expect(billingPlansLink).not.toHaveClass('font-semibold')
    expect(providersLink).toHaveClass('font-semibold')
  })

  it('keeps sidebar navigation in its own scroll container when billing expands', async () => {
    await renderSidebar('/admin/manage/billing?page=0&pageSize=20&status=all')

    const sidebar = screen.getByTestId('admin-sidebar')
    const nav = screen.getByTestId('sidebar-nav')

    expect(sidebar).toHaveClass('h-full', 'min-h-0', 'flex', 'flex-col')
    expect(nav).toHaveClass('min-h-0', 'flex-1', 'overflow-y-auto')
  })
})
