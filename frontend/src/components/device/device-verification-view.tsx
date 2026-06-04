import { useState, useEffect } from 'react'
import { useMutation } from '@tanstack/react-query'
import { deviceVerify, deviceConfirm } from '@/lib/api-generated'
import type { DeviceVerifyResponse } from '@/lib/api-generated'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { CodeInput } from '@/components/device/code-input'
import { AuthorizeConfirm } from '@/components/device/authorize-confirm'
import { getErrorMessage } from '@/lib/error-utils'
import { filterAndFormat, toBackendCode } from './device-code-utils'
import { m } from '@/paraglide/messages'

type PageState = 'input' | 'verifying' | 'confirmed' | 'result'

interface DeviceVerificationViewProps {
  realmId: string
  initialCode?: string
}

export function DeviceVerificationView({ realmId, initialCode }: DeviceVerificationViewProps) {
  const [pageState, setPageState] = useState<PageState>(initialCode ? 'verifying' : 'input')
  const [error, setError] = useState<string | null>(null)
  const [verifyResponse, setVerifyResponse] = useState<DeviceVerifyResponse | null>(null)
  const [resultCode, setResultCode] = useState<'approved' | 'denied' | null>(null)
  const [userCode, setUserCode] = useState(initialCode ?? '')

  const verifyMutation = useMutation({
    mutationFn: async (code: string) => {
      const response = await deviceVerify({
        body: { user_code: code },
        path: { realmId },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (data) => {
      setError(null)
      setVerifyResponse(data)
      setPageState('confirmed')
    },
    onError: (err: unknown) => {
      setError(getErrorMessage(err))
      setPageState('input')
    },
  })

  const confirmMutation = useMutation({
    mutationFn: async (approved: boolean) => {
      const response = await deviceConfirm({
        body: { user_code: userCode, approved },
        path: { realmId },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (data) => {
      setError(null)
      setResultCode(data.status === 'authorized' ? 'approved' : 'denied')
      setPageState('result')
    },
    onError: (err: unknown) => {
      setError(getErrorMessage(err))
    },
  })

  // Auto-submit verify on mount when initialCode is provided
  useEffect(() => {
    if (initialCode) {
      const formatted = filterAndFormat(initialCode)
      setUserCode(formatted)
      verifyMutation.mutate(toBackendCode(formatted))
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialCode])

  function handleCodeSubmit(code: string) {
    setError(null)
    setUserCode(code)
    setPageState('verifying')
    verifyMutation.mutate(code)
  }

  function handleConfirm(approved: boolean) {
    confirmMutation.mutate(approved)
  }

  return (
    <AuthPageWrapper>
      <Card className="w-full max-w-md" data-testid="device-verification-card">
        <CardHeader>
          <CardTitle data-testid="device-verification-title">{m['device.title']()}</CardTitle>
        </CardHeader>
        <CardContent>
          {error && (
            <div
              className="mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-600 text-sm"
              data-testid="device-verification-error"
            >
              {error}
            </div>
          )}

          {pageState === 'input' && (
            <div className="space-y-4">
              <p className="text-sm text-muted-foreground text-center">
                {m['device.enter_code_description']()}
              </p>
              <CodeInput onSubmit={handleCodeSubmit} defaultValue={initialCode} />
            </div>
          )}

          {pageState === 'verifying' && (
            <div className="py-8 text-center text-muted-foreground">
              {m['device.verifying_code']()}
            </div>
          )}

          {pageState === 'confirmed' && verifyResponse && (
            <AuthorizeConfirm
              clientAppName={verifyResponse.client_app_name}
              clientAppIconUrl={verifyResponse.client_app_icon_url}
              onConfirm={handleConfirm}
              isLoading={confirmMutation.isPending}
            />
          )}

          {pageState === 'result' && (
            <div className="py-4 text-center" data-testid="device-verification-result">
              {resultCode === 'approved' ? (
                <div className="space-y-2">
                  <p className="text-green-600 font-medium">
                    {m['device.authorization_successful']()}
                  </p>
                  <p className="text-sm text-muted-foreground">{m['device.return_to_device']()}</p>
                </div>
              ) : (
                <p className="text-red-600 font-medium">{m['device.authorization_denied']()}</p>
              )}
            </div>
          )}
        </CardContent>
      </Card>
    </AuthPageWrapper>
  )
}
