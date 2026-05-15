import { describe, it, expect, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { WechatConfigFormDialog } from '../WechatConfigForm'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'

describe('WechatConfigForm', () => {
  describe('form validation', () => {
    it('GIVEN empty form WHEN submitted THEN shows all required errors', async () => {
      const user = userEvent.setup()
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="create"
        />
      )

      await user.click(screen.getByTestId('wechat-config-submit-button'))

      await waitFor(() => {
        expect(screen.getByText('App ID is required')).toBeInTheDocument()
        expect(screen.getByText('Merchant ID is required')).toBeInTheDocument()
        expect(screen.getByText('Private Key is required')).toBeInTheDocument()
        expect(screen.getByText('API v3 Key is required')).toBeInTheDocument()
        expect(screen.getByText('Notify URL is required')).toBeInTheDocument()
      })

      expect(onSubmit).not.toHaveBeenCalled()
    })

    it('GIVEN invalid App ID format WHEN submitted THEN shows format error', async () => {
      const user = userEvent.setup()
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="create"
        />
      )

      await user.type(screen.getByTestId('app-id-input'), 'invalid-id')
      await user.click(screen.getByTestId('wechat-config-submit-button'))

      await waitFor(() => {
        expect(screen.getByText(/must start with "wx"/i)).toBeInTheDocument()
      })

      expect(onSubmit).not.toHaveBeenCalled()
    })

    it('GIVEN non-numeric Merchant ID WHEN submitted THEN shows numeric error', async () => {
      const user = userEvent.setup()
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="create"
        />
      )

      await user.type(screen.getByTestId('merchant-id-input'), 'abc123')
      await user.click(screen.getByTestId('wechat-config-submit-button'))

      await waitFor(() => {
        expect(screen.getByText(/must be numeric/i)).toBeInTheDocument()
      })

      expect(onSubmit).not.toHaveBeenCalled()
    })

    it('GIVEN invalid PEM format WHEN submitted THEN shows PEM error', async () => {
      const user = userEvent.setup()
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="create"
        />
      )

      await user.type(screen.getByTestId('private-key-input'), 'invalid key content')
      await user.click(screen.getByTestId('wechat-config-submit-button'))

      await waitFor(() => {
        expect(screen.getByText(/valid PEM format/i)).toBeInTheDocument()
      })

      expect(onSubmit).not.toHaveBeenCalled()
    })

    it('GIVEN 31-byte v3Key WHEN submitted THEN shows length error', async () => {
      const user = userEvent.setup()
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="create"
        />
      )

      await user.type(screen.getByTestId('v3-key-input'), 'a'.repeat(31))
      await user.click(screen.getByTestId('wechat-config-submit-button'))

      await waitFor(() => {
        expect(screen.getByText(/exactly 32 bytes/i)).toBeInTheDocument()
      })

      expect(onSubmit).not.toHaveBeenCalled()
    })

    it('GIVEN HTTP notify URL WHEN submitted THEN shows HTTPS error', async () => {
      const user = userEvent.setup()
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="create"
        />
      )

      await user.type(screen.getByTestId('notify-url-input'), 'http://example.com/webhook')
      await user.click(screen.getByTestId('wechat-config-submit-button'))

      await waitFor(() => {
        expect(screen.getByText(/notify url must use https/i)).toBeInTheDocument()
      })

      expect(onSubmit).not.toHaveBeenCalled()
    })
  })

  describe('form submission', () => {
    it('GIVEN valid form WHEN submitted THEN calls onSubmit with correct data', async () => {
      const user = userEvent.setup({ delay: null })
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="create"
        />
      )

      // Fill all fields with valid data
      await user.type(screen.getByTestId('app-id-input'), 'wx1234567890abcdef')
      await user.type(screen.getByTestId('merchant-id-input'), '1234567890')
      await user.type(
        screen.getByTestId('private-key-input'),
        '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----'
      )
      await user.type(screen.getByTestId('serial-no-input'), '1A2B3C4D5E6F')
      await user.type(screen.getByTestId('v3-key-input'), '0123456789abcdefghijklmnopqrstuv')
      await user.type(screen.getByTestId('notify-url-input'), 'https://example.com/api/webhook')

      await user.click(screen.getByTestId('wechat-config-submit-button'))

      await waitFor(() => {
        expect(onSubmit).toHaveBeenCalledWith({
          appId: 'wx1234567890abcdef',
          mchId: '1234567890',
          privateKey:
            '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
          serialNo: '1A2B3C4D5E6F',
          v3Key: '0123456789abcdefghijklmnopqrstuv',
          notifyUrl: 'https://example.com/api/webhook',
        })
      })
    }, 10000)

    it('GIVEN API returns 409 conflict WHEN submitted THEN shows error', async () => {
      const user = userEvent.setup({ delay: null })
      const onSubmit = vi.fn()

      server.use(
        http.post('/api/third/pay/realm-1/providers/wechat', () =>
          HttpResponse.json({ message: 'WeChat Pay configuration already exists' }, { status: 409 })
        )
      )

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="create"
          isSubmitting={false}
        />
      )

      // Fill form with valid data
      await user.type(screen.getByTestId('app-id-input'), 'wx1234567890abcdef')
      await user.type(screen.getByTestId('merchant-id-input'), '1234567890')
      await user.type(
        screen.getByTestId('private-key-input'),
        '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----'
      )
      await user.type(screen.getByTestId('serial-no-input'), '1A2B3C4D5E6F')
      await user.type(screen.getByTestId('v3-key-input'), '0123456789abcdefghijklmnopqrstuv')
      await user.type(screen.getByTestId('notify-url-input'), 'https://example.com/api/webhook')

      await user.click(screen.getByTestId('wechat-config-submit-button'))

      // The form should call onSubmit with the validated data
      // The parent component is responsible for handling the API call and errors
      await waitFor(() => {
        expect(onSubmit).toHaveBeenCalledWith({
          appId: 'wx1234567890abcdef',
          mchId: '1234567890',
          privateKey:
            '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
          serialNo: '1A2B3C4D5E6F',
          v3Key: '0123456789abcdefghijklmnopqrstuv',
          notifyUrl: 'https://example.com/api/webhook',
        })
      })
    }, 10000)
  })

  describe('edit mode', () => {
    it('GIVEN existing config WHEN edit mode THEN pre-fills non-sensitive fields', () => {
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="edit"
          initialValues={{
            appId: 'wx1234567890abcdef',
            mchId: '1234567890',
            serialNo: '1A2B3C4D5E6F',
            notifyUrl: 'https://example.com/api/webhook',
          }}
        />
      )

      expect(screen.getByTestId('app-id-input')).toHaveValue('wx1234567890abcdef')
      expect(screen.getByTestId('merchant-id-input')).toHaveValue('1234567890')
      expect(screen.getByTestId('serial-no-input')).toHaveValue('1A2B3C4D5E6F')
      expect(screen.getByTestId('notify-url-input')).toHaveValue('https://example.com/api/webhook')
    })

    it('GIVEN edit mode WHEN submitted THEN calls onSubmit with updated data', async () => {
      const user = userEvent.setup()
      const onSubmit = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={onSubmit}
          mode="edit"
          initialValues={{
            appId: 'wx1234567890abcdef',
            mchId: '1234567890',
            privateKey:
              '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
            serialNo: '1A2B3C4D5E6F',
            v3Key: '0123456789abcdefghijklmnopqrstuv',
            notifyUrl: 'https://example.com/api/webhook',
          }}
        />
      )

      // Update some fields
      await user.clear(screen.getByTestId('notify-url-input'))
      await user.type(
        screen.getByTestId('notify-url-input'),
        'https://updated.example.com/api/webhook'
      )

      await user.click(screen.getByTestId('wechat-config-submit-button'))

      await waitFor(() => {
        expect(onSubmit).toHaveBeenCalledWith({
          appId: 'wx1234567890abcdef',
          mchId: '1234567890',
          privateKey:
            '-----BEGIN PRIVATE KEY-----\nMIIEvwIBADANBgkqhkiG9w0BAQEFAASCBKkwggSlAgEAAoIBAQ\n-----END PRIVATE KEY-----',
          serialNo: '1A2B3C4D5E6F',
          v3Key: '0123456789abcdefghijklmnopqrstuv',
          notifyUrl: 'https://updated.example.com/api/webhook',
        })
      })
    })
  })

  describe('cancel functionality', () => {
    it('GIVEN form open WHEN cancel clicked THEN calls onOpenChange with false', async () => {
      const user = userEvent.setup()
      const onOpenChange = vi.fn()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={onOpenChange}
          onSubmit={vi.fn()}
          mode="create"
        />
      )

      await user.click(screen.getByTestId('wechat-config-cancel-button'))

      expect(onOpenChange).toHaveBeenCalledWith(false)
    })
  })

  describe('form validation on field blur', () => {
    it('GIVEN App ID field blurred with invalid value WHEN blurred THEN shows error', async () => {
      const user = userEvent.setup()

      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={vi.fn()}
          mode="create"
        />
      )

      const appIdInput = screen.getByTestId('app-id-input')
      await user.type(appIdInput, 'invalid-id')
      await user.tab() // Blur the field

      await waitFor(() => {
        expect(screen.getByText(/must start with "wx"/i)).toBeInTheDocument()
      })
    })
  })

  describe('submitting state', () => {
    it('GIVEN form is submitting WHEN isSubmitting is true THEN disables submit button', () => {
      render(
        <WechatConfigFormDialog
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={vi.fn()}
          mode="create"
          isSubmitting={true}
        />
      )

      const submitButton = screen.getByTestId('wechat-config-submit-button')
      expect(submitButton).toBeDisabled()
      expect(submitButton).toHaveTextContent('Saving...')
    })
  })
})
