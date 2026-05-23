/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Suspense } from 'react'
import type { ReactNode } from 'react'

// Mock usePermission to control permission state per test
vi.mock('@/hooks/use-permission', () => ({
  usePermission: vi.fn(),
}))

// Mock TanStack Router hooks used by the page component
vi.mock('@tanstack/react-router', () => ({
  createFileRoute: () => {
    const routeObj = {
      useParams: () => ({ realmId: 'test-realm' }),
      useSearch: () => ({ page: 0, pageSize: 20 }),
    }
    return (config: Record<string, unknown>) => ({ ...routeObj, ...config })
  },
  useNavigate: () => vi.fn(),
}))

// Mock query options so the page does not fire real API calls
vi.mock('@/data/query-options', () => ({
  apiKeysQueryOptions: () => ({
    queryKey: ['api-keys', 'test-realm'],
    queryFn: () => Promise.resolve({ items: [], page: 0, pageSize: 20, total: 0 }),
  }),
  queryKeys: {
    apiKeysList: () => ['api-keys', 'test-realm'],
  },
}))

// Mock form mutation hooks
vi.mock('@/hooks/use-form-mutation', () => ({
  useFormMutation: () => ({
    mutate: vi.fn(),
    isSubmitting: false,
  }),
}))

// Mock dialog manager hook
vi.mock('@/hooks/use-dialog-state', () => ({
  useDialogManager: () => ({
    selectedItem: null,
    isOpen: false,
    open: vi.fn(),
    close: vi.fn(),
    onOpenChange: vi.fn(),
  }),
}))

// Mock child components to isolate permission gating logic
vi.mock('@/components/api-keys/api-key-table', () => ({
  ApiKeyTable: ({ canUpdate, canDelete }: { canUpdate: boolean; canDelete: boolean }) => (
    <div data-testid="api-key-table">
      <span data-testid="can-update">{String(canUpdate)}</span>
      <span data-testid="can-delete">{String(canDelete)}</span>
    </div>
  ),
}))

vi.mock('@/components/api-keys/delete-api-key-dialog', () => ({
  DeleteApiKeyDialog: () => <div data-testid="delete-dialog" />,
}))

// Mock UI components
vi.mock('@/components/ui/card', () => ({
  Card: ({ children }: { children: ReactNode }) => <div data-testid="card">{children}</div>,
  CardContent: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}))

vi.mock('@/components/shared', () => ({
  PageHeader: ({
    title,
    headingTestId,
    action,
  }: {
    title: string
    headingTestId?: string
    action?: { label: string; onClick: () => void; testId: string; icon?: ReactNode }
  }) => (
    <div data-testid="page-header">
      <h1 data-testid={headingTestId ?? 'heading'}>{title}</h1>
      {action && <button data-testid={action.testId}>{action.label}</button>}
    </div>
  ),
  ListPagination: () => <div data-testid="pagination" />,
  AccessDenied: ({ message }: { message?: string }) => (
    <div className="text-destructive">
      {message ?? 'Access denied: You do not have permission to view this page'}
    </div>
  ),
}))

vi.mock('lucide-react', () => ({
  Plus: () => <span>+</span>,
}))

import { usePermission } from '@/hooks/use-permission'
import { ApiKeysPage } from '../index'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  })
}

function renderPage() {
  const queryClient = createTestQueryClient()
  return render(
    <QueryClientProvider client={queryClient}>
      <Suspense fallback={<div data-testid="loading">Loading...</div>}>
        <ApiKeysPage />
      </Suspense>
    </QueryClientProvider>
  )
}

describe('API Keys page permission gating', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows access-denied when user has no api_keys.view permission', async () => {
    vi.mocked(usePermission).mockReturnValue({
      hasPermission: (permission: string) => false,
      hasAnyPermission: vi.fn(),
      hasAllPermissions: vi.fn(),
      hasRole: vi.fn(),
      hasAnyRole: vi.fn(),
      hasAdminPermission: false,

      permissions: [],
      roles: [],
      isLoading: false,
    })

    renderPage()

    expect(await screen.findByText(/access denied/i)).toBeInTheDocument()
    expect(screen.getByText(/do not have permission to view api keys/i)).toBeInTheDocument()
    expect(screen.queryByTestId('api-keys-page')).not.toBeInTheDocument()
  })

  it('shows page content but hides manage buttons when user has only api_keys.view', async () => {
    vi.mocked(usePermission).mockReturnValue({
      hasPermission: (permission: string) => permission === 'api_keys.view',
      hasAnyPermission: vi.fn(),
      hasAllPermissions: vi.fn(),
      hasRole: vi.fn(),
      hasAnyRole: vi.fn(),
      hasAdminPermission: false,

      permissions: ['api_keys.view'],
      roles: [],
      isLoading: false,
    })

    renderPage()

    // Page is visible (not access-denied)
    await waitFor(() => {
      expect(screen.getByTestId('api-keys-page')).toBeInTheDocument()
    })
    expect(screen.queryByText(/access denied/i)).not.toBeInTheDocument()

    // Add API Key button is not rendered (action is undefined when !canManage)
    expect(screen.queryByTestId('add-api-key-button')).not.toBeInTheDocument()

    // Table is rendered with canUpdate=false and canDelete=false
    expect(screen.getByTestId('can-update')).toHaveTextContent('false')
    expect(screen.getByTestId('can-delete')).toHaveTextContent('false')
  })

  it('shows page content and manage buttons when user has both api_keys.view and api_keys.manage', async () => {
    vi.mocked(usePermission).mockReturnValue({
      hasPermission: (permission: string) =>
        permission === 'api_keys.view' || permission === 'api_keys.manage',
      hasAnyPermission: vi.fn(),
      hasAllPermissions: vi.fn(),
      hasRole: vi.fn(),
      hasAnyRole: vi.fn(),
      hasAdminPermission: false,

      permissions: ['api_keys.view', 'api_keys.manage'],
      roles: [],
      isLoading: false,
    })

    renderPage()

    // Wait for the page to render
    await waitFor(() => {
      expect(screen.getByTestId('api-keys-page')).toBeInTheDocument()
    })

    // Page is visible
    expect(screen.queryByText(/access denied/i)).not.toBeInTheDocument()

    // Add API Key button is rendered
    expect(screen.getByTestId('add-api-key-button')).toBeInTheDocument()

    // Table receives canUpdate=true and canDelete=true
    expect(screen.getByTestId('can-update')).toHaveTextContent('true')
    expect(screen.getByTestId('can-delete')).toHaveTextContent('true')
  })

  it('shows access-denied when user has only api_keys.manage but not api_keys.view', async () => {
    vi.mocked(usePermission).mockReturnValue({
      hasPermission: (permission: string) => permission === 'api_keys.manage',
      hasAnyPermission: vi.fn(),
      hasAllPermissions: vi.fn(),
      hasRole: vi.fn(),
      hasAnyRole: vi.fn(),
      hasAdminPermission: false,

      permissions: ['api_keys.manage'],
      roles: [],
      isLoading: false,
    })

    renderPage()

    // Frontend checks view independently; manage-only is still denied
    expect(await screen.findByText(/access denied/i)).toBeInTheDocument()
    expect(screen.getByText(/do not have permission to view api keys/i)).toBeInTheDocument()
    expect(screen.queryByTestId('api-keys-page')).not.toBeInTheDocument()
  })
})
