import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { ClientAppTable } from '../client-app-table'
import type { ClientAppItem } from '@/lib/api-generated'

const mockClientApps: ClientAppItem[] = [
  {
    id: '1',
    realmId: 'test-realm-1',
    clientId: 'app-1',
    name: 'Application 1',
    description: 'First application',
    redirectUris: ['https://app1.example.com/callback'],
    iconUrl: 'https://app1.example.com/icon.png',
    enabled: true,
    sessionTtlSeconds: 1800,
    sessionRenewalTtlSeconds: null,
  },
  {
    id: '2',
    realmId: 'test-realm-2',
    clientId: 'app-2',
    name: 'Application 2',
    description: 'Second application',
    redirectUris: ['https://app2.example.com/callback'],
    iconUrl: null,
    enabled: false,
    sessionTtlSeconds: 3600,
    sessionRenewalTtlSeconds: 7200,
  },
]

describe('ClientAppTable', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN data is provided WHEN rendering THEN should display table', async () => {
    const screen = render(
      <ClientAppTable
        data={mockClientApps}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    expect(screen.getByTestId('client-apps-table')).toBeInTheDocument()
  })

  it('GIVEN client apps exist WHEN rendering THEN should display all rows', async () => {
    render(
      <ClientAppTable
        data={mockClientApps}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    const row0 = document.querySelector('[data-testid="client-app-row-0"]')
    const row1 = document.querySelector('[data-testid="client-app-row-1"]')
    expect(row0).not.toBeNull()
    expect(row1).not.toBeNull()
  })

  it('GIVEN client app has icon WHEN rendering THEN should display icon', async () => {
    render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    const icon = document.querySelector('img[data-testid="client-app-icon"]') as HTMLImageElement
    expect(icon).not.toBeNull()
    expect(icon?.src).toContain('app1.example.com/icon.png')
  })

  it('GIVEN client app has no icon WHEN rendering THEN should display N/A placeholder', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[1]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    expect(screen.getByText('N/A')).toBeInTheDocument()
  })

  it('GIVEN client app WHEN rendering THEN should display client ID', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    expect(screen.getByTestId('client-app-client-id')).toHaveTextContent('app-1')
  })

  it('GIVEN client app WHEN rendering THEN should display name', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    expect(screen.getByText('Application 1')).toBeInTheDocument()
  })

  it('GIVEN client app WHEN rendering THEN should display redirect URIs', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    expect(screen.getByTestId('client-app-redirect-uris')).toHaveTextContent(
      'https://app1.example.com/callback'
    )
  })

  it('GIVEN client app WHEN rendering THEN should display session TTL', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    expect(screen.getByTestId('client-app-session-ttl')).toHaveTextContent('30 min')
  })

  it('GIVEN client app is enabled WHEN rendering THEN should display enabled badge', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    const badge = screen.getByTestId('client-app-status-badge')
    expect(badge).toHaveTextContent('Enabled')
  })

  it('GIVEN client app is disabled WHEN rendering THEN should display disabled badge', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[1]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    const badge = screen.getByTestId('client-app-status-badge')
    expect(badge).toHaveTextContent('Disabled')
  })

  it('GIVEN client app is enabled WHEN rendering THEN should display checked switch', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    const switchElement = screen.getByTestId('client-app-enabled-switch')
    expect(switchElement).toBeChecked()
  })

  it('GIVEN client app is disabled WHEN rendering THEN should display unchecked switch', async () => {
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[1]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    const switchElement = screen.getByTestId('client-app-enabled-switch')
    expect(switchElement).not.toBeChecked()
  })

  it('GIVEN user clicks enabled switch WHEN clicked THEN should call onToggleEnabled', async () => {
    const onToggleEnabled = vi.fn()
    render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={onToggleEnabled}
      />
    )

    const switchElement = document.querySelector(
      '[data-testid="client-app-enabled-switch"]'
    ) as HTMLElement
    await userEvent.click(switchElement)

    expect(onToggleEnabled).toHaveBeenCalledWith(mockClientApps[0])
  })

  it('GIVEN user clicks Edit button WHEN clicked THEN should call onEdit', async () => {
    const onEdit = vi.fn()
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={onEdit}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    const editButton = screen.getByTestId('edit-client-app-button')
    await userEvent.click(editButton)

    expect(onEdit).toHaveBeenCalledWith(mockClientApps[0])
  })

  it('GIVEN user clicks Delete button WHEN clicked THEN should call onDelete', async () => {
    const onDelete = vi.fn()
    const screen = render(
      <ClientAppTable
        data={[mockClientApps[0]]}
        onEdit={vi.fn()}
        onDelete={onDelete}
        onToggleEnabled={vi.fn()}
      />
    )

    const deleteButton = screen.getByTestId('delete-client-app-button')
    await userEvent.click(deleteButton)

    expect(onDelete).toHaveBeenCalledWith(mockClientApps[0])
  })

  it('GIVEN loading WHEN rendering THEN should display loading message', async () => {
    const screen = render(
      <ClientAppTable
        data={[]}
        isLoading={true}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    expect(screen.getByText('Loading client apps...')).toBeInTheDocument()
  })

  it('GIVEN error occurs WHEN rendering THEN should display error message', async () => {
    const error = new Error('Failed to load')
    const screen = render(
      <ClientAppTable
        data={[]}
        error={error}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    expect(screen.getByText(/Error loading client apps:/)).toBeInTheDocument()
    expect(screen.getByText('Failed to load', { exact: false })).toBeInTheDocument()
  })

  it('GIVEN no data WHEN rendering THEN should display empty message', async () => {
    const screen = render(
      <ClientAppTable data={[]} onEdit={vi.fn()} onDelete={vi.fn()} onToggleEnabled={vi.fn()} />
    )

    expect(screen.getByText(/No client apps found/)).toBeInTheDocument()
  })

  it('GIVEN multiple redirect URIs WHEN rendering THEN should display all URIs', async () => {
    const multiUriApp: ClientAppItem = {
      ...mockClientApps[0],
      redirectUris: ['https://app1.example.com/callback', 'https://app1.example.com/redirect'],
    }

    const screen = render(
      <ClientAppTable
        data={[multiUriApp]}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onToggleEnabled={vi.fn()}
      />
    )

    const urisElement = screen.getByTestId('client-app-redirect-uris')
    expect(urisElement).toHaveTextContent(
      'https://app1.example.com/callback, https://app1.example.com/redirect'
    )
  })
})
