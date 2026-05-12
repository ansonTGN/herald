import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Loader2, CheckCircle2, XCircle } from 'lucide-react'
import type { ShopifyConfigForm } from '@/lib/schemas/billing-forms'

interface ConnectionStatusRowProps {
  label: string
  success: boolean
  message?: string
  dataTestId: string
}

function ConnectionStatusRow({ label, success, message, dataTestId }: ConnectionStatusRowProps) {
  return (
    <div
      className="flex items-center justify-between p-3 border rounded-lg"
      data-testid={dataTestId}
    >
      <div className="flex-1">
        <div className="font-medium">{label}</div>
        {message && <div className="text-sm text-muted-foreground mt-1">{message}</div>}
      </div>
      <Badge variant={success ? 'default' : 'destructive'}>{success ? 'Success' : 'Failed'}</Badge>
    </div>
  )
}

interface TestConnectionDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  config: ShopifyConfigForm
  onTest: (config: ShopifyConfigForm) => Promise<TestResult>
}

export interface TestResult {
  adminApiSuccess: boolean
  adminApiMessage?: string
  storefrontApiSuccess: boolean
  storefrontApiMessage?: string
  shopAccessSuccess: boolean
  shopAccessMessage?: string
}

export function TestConnectionDialog({
  open,
  onOpenChange,
  config,
  onTest,
}: TestConnectionDialogProps) {
  const [isTesting, setIsTesting] = useState(false)
  const [result, setResult] = useState<TestResult | null>(null)

  const handleTest = async () => {
    setIsTesting(true)
    setResult(null)

    try {
      const testResult = await onTest(config)
      setResult(testResult)
    } catch (error) {
      setResult({
        adminApiSuccess: false,
        adminApiMessage: error instanceof Error ? error.message : 'Unknown error',
        storefrontApiSuccess: false,
        storefrontApiMessage: error instanceof Error ? error.message : 'Unknown error',
        shopAccessSuccess: false,
        shopAccessMessage: error instanceof Error ? error.message : 'Unknown error',
      })
    } finally {
      setIsTesting(false)
    }
  }

  const handleClose = () => {
    setResult(null)
    onOpenChange(false)
  }

  const allSuccess =
    result && result.adminApiSuccess && result.storefrontApiSuccess && result.shopAccessSuccess

  return (
    <BaseFormDialog
      open={open}
      onOpenChange={handleClose}
      title="Test Shopify Connection"
      description="Testing connection to Shopify API"
      className="max-w-lg"
      data-testid="test-connection-dialog"
      footer={
        <Button
          type="button"
          onClick={handleClose}
          disabled={isTesting}
          data-testid="test-connection-close-button"
        >
          Close
        </Button>
      }
    >
      <div className="space-y-4">
        {!result && !isTesting && (
          <div className="text-center py-4">
            <p className="text-sm text-muted-foreground mb-4">
              This will test your Shopify configuration by connecting to the Admin API, Storefront
              API, and verifying shop access.
            </p>
            <Button type="button" onClick={handleTest} data-testid="start-test-button">
              Start Test
            </Button>
          </div>
        )}

        {isTesting && (
          <div
            className="flex flex-col items-center justify-center py-8"
            data-testid="testing-indicator"
          >
            <Loader2 className="h-8 w-8 animate-spin text-primary mb-4" />
            <p className="text-sm text-muted-foreground">Testing connection...</p>
          </div>
        )}

        {result && (
          <div className="space-y-4">
            {allSuccess && (
              <Alert className="border-green-200 bg-green-50">
                <CheckCircle2 className="h-4 w-4 text-green-600" />
                <AlertDescription className="text-green-800">
                  All connection tests passed successfully!
                </AlertDescription>
              </Alert>
            )}

            <div className="space-y-3">
              <ConnectionStatusRow
                label="Admin API"
                success={result.adminApiSuccess}
                message={result.adminApiMessage}
                dataTestId="connection-status-admin-api"
              />

              <ConnectionStatusRow
                label="Storefront API"
                success={result.storefrontApiSuccess}
                message={result.storefrontApiMessage}
                dataTestId="connection-status-storefront-api"
              />

              <ConnectionStatusRow
                label="Shop Access"
                success={result.shopAccessSuccess}
                message={result.shopAccessMessage}
                dataTestId="connection-status-shop-access"
              />
            </div>

            {!allSuccess && (
              <Alert className="border-destructive bg-destructive/10">
                <XCircle className="h-4 w-4" />
                <AlertDescription>
                  Some connection tests failed. Please check your configuration and try again.
                </AlertDescription>
              </Alert>
            )}
          </div>
        )}
      </div>
    </BaseFormDialog>
  )
}
