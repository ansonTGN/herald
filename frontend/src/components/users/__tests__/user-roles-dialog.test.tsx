import { describe, it, expect, afterEach, beforeEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { UserRolesDialog } from '../user-roles-dialog'

const mockRolesData = [
  { id: '1', name: 'realm-admin' },
  { id: '2', name: 'user' },
  { id: '3', name: 'custom-role' },
]

const mockUserRolesData = {
  roles: [
    { id: '1', name: 'realm-admin' },
    { id: '2', name: 'user' },
  ],
}

const mockUseQuery = vi.fn()
const mockInvalidateQueries = vi.fn()
const mockUpdateUserRoles = vi.fn()

vi.mock('@tanstack/react-query', () => ({
  useQuery: (options: any) => mockUseQuery(options),
  useQueryClient: () => ({
    invalidateQueries: mockInvalidateQueries,
  }),
  queryOptions: vi.fn((options) => options),
}))

const mockRolesQueryOptions = { queryKey: ['roles', 'realm-1'] }
const mockUserRolesQueryOptions = { queryKey: ['admin-user-roles', 'realm-1', 'user-1'] }

vi.mock('@/data/query-options', () => ({
  rolesQueryOptions: () => mockRolesQueryOptions,
  adminUserRolesQueryOptions: () => mockUserRolesQueryOptions,
  queryKeys: {
    adminUserRoles: () => ['admin-user-roles', 'realm-1', 'user-1'],
    usersList: () => ['users', 'realm-1'],
  },
}))

vi.mock('@/lib/api-generated', () => ({
  updateUserRoles: (...args: any[]) => mockUpdateUserRoles(...args),
}))

vi.mock('@/stores/auth-store', () => ({
  useRealmId: () => 'realm-1',
}))

beforeEach(() => {
  mockUseQuery.mockImplementation((options) => {
    if (Array.isArray(options.queryKey) && options.queryKey[0] === 'roles') {
      return {
        data: mockRolesData,
        isLoading: false,
        isPending: false,
        isSuccess: true,
      }
    }
    if (Array.isArray(options.queryKey) && options.queryKey[0] === 'admin-user-roles') {
      return {
        data: { data: mockUserRolesData },
        isLoading: false,
        isPending: false,
        isSuccess: true,
      }
    }
    return {
      data: null,
      isLoading: false,
      isPending: false,
      isSuccess: false,
    }
  })
})

afterEach(() => {
  vi.clearAllMocks()
})

describe('UserRolesDialog', () => {
  const mockUserId = 'user-1'
  const mockUserEmail = 'user@example.com'

  it('GIVEN user clicks Cancel button WHEN clicked THEN should call onOpenChange with false', async () => {
    const handleOpenChange = vi.fn()
    render(
      <UserRolesDialog
        open={true}
        onOpenChange={handleOpenChange}
        userId={mockUserId}
        userEmail={mockUserEmail}
      />
    )

    const cancelButton = document.querySelector(
      '[data-testid="user-roles-dialog-cancel"]'
    ) as HTMLElement
    await userEvent.click(cancelButton)

    expect(handleOpenChange).toHaveBeenCalledTimes(1)
    expect(handleOpenChange).toHaveBeenCalledWith(false)
  })

  it('GIVEN user clicks overlay WHEN clicked THEN should call onOpenChange with false', async () => {
    const handleOpenChange = vi.fn()
    render(
      <UserRolesDialog
        open={true}
        onOpenChange={handleOpenChange}
        userId={mockUserId}
        userEmail={mockUserEmail}
      />
    )

    const overlay = document.querySelector('[data-slot="dialog-overlay"]') as HTMLElement
    await userEvent.click(overlay)

    expect(handleOpenChange).toHaveBeenCalledTimes(1)
    expect(handleOpenChange).toHaveBeenCalledWith(false)
  })

  it('GIVEN roles are loading WHEN rendering THEN should show loading state', async () => {
    mockUseQuery.mockImplementation(() => ({
      data: null,
      isLoading: true,
      isPending: true,
      isSuccess: false,
    }))

    const screen = render(
      <UserRolesDialog
        open={true}
        onOpenChange={vi.fn()}
        userId={mockUserId}
        userEmail={mockUserEmail}
      />
    )

    expect(screen.getByText('Loading roles...')).toBeInTheDocument()
  })

  it('GIVEN user clicks content WHEN clicked THEN should not close dialog', async () => {
    const handleOpenChange = vi.fn()
    render(
      <UserRolesDialog
        open={true}
        onOpenChange={handleOpenChange}
        userId={mockUserId}
        userEmail={mockUserEmail}
      />
    )

    const content = document.querySelector(
      '[data-testid="user-roles-dialog-content"]'
    ) as HTMLElement
    await userEvent.click(content)

    expect(handleOpenChange).not.toHaveBeenCalled()
  })
})
