import { useEffect, useCallback } from 'react'
import { useRouterState, useNavigate } from '@tanstack/react-router'
import type { CreateApiKeyResponse } from '@/lib/api-generated'
import { Button } from '@/components/ui/button'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Card, CardContent } from '@/components/ui/card'
import { Copy, Check, TriangleAlert, ArrowLeft } from 'lucide-react'
import { useCopyToClipboard } from '@/hooks/use-copy-to-clipboard'
import { m } from '@/paraglide/messages'

interface ApiKeyRevealPageProps {
  realmId: string
}

export function ApiKeyRevealPage({ realmId }: ApiKeyRevealPageProps) {
  const navigate = useNavigate()
  const { copied, copyToClipboard } = useCopyToClipboard()

  const keyData = useRouterState({
    select: (s) =>
      (s.location.state as unknown as Record<string, unknown> | undefined)?.keyData as
        | CreateApiKeyResponse
        | undefined,
  })

  // Carried over from the create flow when the post-creation role binding failed.
  // Shown persistently here instead of a fleeting toast so the failure isn't swallowed.
  const roleBindingError = useRouterState({
    select: (s) =>
      (s.location.state as unknown as Record<string, unknown> | undefined)?.roleBindingError as
        | string
        | undefined,
  })

  useEffect(() => {
    if (!keyData) {
      void navigate({ to: '/$realmId/manage/api-keys', params: { realmId } })
    }
  }, [keyData, navigate, realmId])

  const handleCopy = useCallback(async () => {
    if (!keyData?.key) return
    await copyToClipboard(keyData.key)
  }, [keyData, copyToClipboard])

  const handleDone = () => {
    void navigate({ to: '/$realmId/manage/api-keys', params: { realmId } })
  }

  if (!keyData) {
    return null
  }

  return (
    <div className="space-y-6" data-testid="api-key-reveal-page">
      <div className="flex items-center gap-4">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleDone}
          data-testid="api-key-reveal-back-button"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div>
          <h1 className="text-2xl font-bold" data-testid="page-title">
            {m['api_keys.reveal_title']()}
          </h1>
          <p className="text-muted-foreground text-sm">{m['api_keys.reveal_subtitle']()}</p>
        </div>
      </div>

      {roleBindingError && (
        <Alert variant="destructive">
          <TriangleAlert className="h-4 w-4" />
          <AlertDescription>
            <p className="font-medium">{m['api_keys.role_binding_failed']()}</p>
            <p className="mt-1 break-words">{roleBindingError}</p>
          </AlertDescription>
        </Alert>
      )}

      <Alert className="border-yellow-500/50 bg-yellow-50 text-yellow-800 dark:bg-yellow-950 dark:text-yellow-200">
        <TriangleAlert className="h-4 w-4 !text-yellow-600 dark:!text-yellow-400" />
        <AlertDescription>{m['api_keys.reveal_warning']()}</AlertDescription>
      </Alert>

      <Card>
        <CardContent className="pt-6 space-y-4">
          <div className="space-y-1">
            <p className="text-sm font-medium text-muted-foreground">
              {m['api_keys.reveal_name_label']()}
            </p>
            <p className="text-sm font-semibold">{keyData.name}</p>
          </div>

          <div className="space-y-2">
            <p className="text-sm font-medium text-muted-foreground">
              {m['api_keys.reveal_key_label']()}
            </p>
            <div className="flex items-center gap-2">
              <code
                className="flex-1 rounded-md bg-muted px-3 py-2 font-mono text-sm break-all select-all"
                data-testid="api-key-reveal-value"
              >
                {keyData.key}
              </code>
              <Button
                variant="outline"
                size="sm"
                onClick={handleCopy}
                className="shrink-0"
                data-testid="copy-api-key-button"
              >
                {copied ? (
                  <>
                    <Check className="h-4 w-4 mr-2" />
                    {m['api_keys.reveal_copied']()}
                  </>
                ) : (
                  <>
                    <Copy className="h-4 w-4 mr-2" />
                    {m['api_keys.reveal_copy']()}
                  </>
                )}
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="flex justify-end">
        <Button onClick={handleDone} data-testid="api-key-reveal-done-button">
          {m['api_keys.reveal_done']()}
        </Button>
      </div>
    </div>
  )
}
