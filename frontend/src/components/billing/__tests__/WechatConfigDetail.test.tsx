import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { WechatConfigDetail } from '../WechatConfigDetail'

describe('WechatConfigDetail', () => {
  const mockConfig = {
    platform: 'wechat',
    appId: 'wx1234567890abcdef',
    mchId: '1234567890',
    serialNo: 'ABC123DEF456',
    v3Key: 'my_v3_secret_key_here_12345',
    privateKey:
      '-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC=\n-----END PRIVATE KEY-----',
    notifyUrl: 'https://example.com/api/wechat/notify',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T12:00:00Z',
  }

  it('GIVEN masked config WHEN rendered THEN displays masked values', () => {
    const onShowSecrets = vi.fn()
    const onHideSecrets = vi.fn()
    const onEdit = vi.fn()
    const onDelete = vi.fn()

    render(
      <WechatConfigDetail
        config={mockConfig}
        onShowSecrets={onShowSecrets}
        onHideSecrets={onHideSecrets}
        showSecrets={false}
        onEdit={onEdit}
        onDelete={onDelete}
      />
    )

    // Verify non-sensitive fields are displayed
    expect(screen.getByTestId('app-id-display')).toHaveTextContent('wx1234567890abcdef')
    expect(screen.getByTestId('merchant-id-display')).toHaveTextContent('1234567890')
    expect(screen.getByTestId('serial-no-display')).toHaveTextContent('ABC123DEF456')
    expect(screen.getByTestId('notify-url-display')).toHaveTextContent(
      'https://example.com/api/wechat/notify'
    )

    // Verify show secrets button is present
    expect(screen.getByTestId('show-secrets-button')).toBeInTheDocument()
    expect(screen.queryByTestId('hide-secrets-button')).not.toBeInTheDocument()
  })

  it('GIVEN showSecrets=true WHEN rendered THEN displays hide secrets button', () => {
    const onShowSecrets = vi.fn()
    const onHideSecrets = vi.fn()
    const onEdit = vi.fn()
    const onDelete = vi.fn()

    render(
      <WechatConfigDetail
        config={mockConfig}
        onShowSecrets={onShowSecrets}
        onHideSecrets={onHideSecrets}
        showSecrets={true}
        onEdit={onEdit}
        onDelete={onDelete}
      />
    )

    expect(screen.getByTestId('hide-secrets-button')).toBeInTheDocument()
    expect(screen.queryByTestId('show-secrets-button')).not.toBeInTheDocument()
  })
})
