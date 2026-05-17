import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { PageHeader } from '@/components/shared/page-header'
import { ShopifyConfigDetail } from './ShopifyConfigDetail'
import { WechatConfigDetail } from './WechatConfigDetail'
import { DeleteConfirmDialog } from './DeleteConfirmDialog'
import { Edit, Trash2, Plug2, Plus } from 'lucide-react'
import {
  deleteShopifyConfig,
  getShopifyConfig,
  listPaymentProviders,
  deleteWechatConfig,
  getWechatConfig,
  type ShopifyConfigResponse,
  type WechatConfigResponse,
} from '@/lib/api-generated'
import { listRealmConfigs, deleteRealmConfig } from '@/lib/api-generated/sdk.gen'
import { parseStripeConfig } from '@/lib/stripe-config-utils'
import { parseCreemConfig, CREEM_CONFIG_KEYS } from '@/lib/creem-config-utils'

interface ProviderCardProps {
  name: string
  description: string
  enabled: boolean
  providerType: 'shopify' | 'wechat' | 'stripe' | 'creem'
  onEdit: (type: 'shopify' | 'wechat' | 'stripe' | 'creem') => void
  onDelete: () => void
}

function ProviderCard({ name, description, enabled, providerType, onEdit, onDelete }: ProviderCardProps) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>{name}</CardTitle>
            <CardDescription>
              {description} &middot; {enabled ? 'Enabled' : 'Disabled'}
            </CardDescription>
          </div>
          <Badge variant={enabled ? 'default' : 'secondary'}>
            {enabled ? 'Active' : 'Disabled'}
          </Badge>
        </div>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onEdit(providerType)}
            data-testid={`edit-${providerType}-button`}
          >
            <Edit className="mr-2 h-4 w-4" />
            Edit
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={onDelete}
            data-testid={`delete-${providerType}-button`}
          >
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}

interface PaymentProvidersPageProps {
  realmId: string
}

export function PaymentProvidersPage({ realmId }: PaymentProvidersPageProps) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false)
  const [deleteProviderType, setDeleteProviderType] = useState<
    'shopify' | 'wechat' | 'stripe' | 'creem'
  >('shopify')
  const [shopifyConfigDetails, setShopifyConfigDetails] = useState<ShopifyConfigResponse | null>(
    null
  )
  const [wechatConfigDetails, setWechatConfigDetails] = useState<WechatConfigResponse | null>(null)
  const [stripeConfigDetails, setStripeConfigDetails] = useState<{ enabled: boolean } | null>(null)
  const [creemConfigDetails, setCreemConfigDetails] = useState<{ enabled: boolean } | null>(null)
  const [showWechatSecrets, setShowWechatSecrets] = useState(false)

  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout>
    if (showWechatSecrets) {
      timeoutId = setTimeout(() => {
        setShowWechatSecrets(false)
      }, 5000)
    }
    return () => {
      if (timeoutId) clearTimeout(timeoutId)
    }
  }, [showWechatSecrets])

  const { data: providers, isLoading } = useQuery({
    queryKey: ['payment-providers', realmId],
    queryFn: async () => {
      const result = await listPaymentProviders({
        path: { realmId },
      })
      return result.data?.providers ?? []
    },
  })

  useEffect(() => {
    const shopifyProvider = providers?.find((p) => p.platform === 'shopify')
    if (shopifyProvider) {
      getShopifyConfig({ path: { realmId } })
        .then((result) => setShopifyConfigDetails(result.data as ShopifyConfigResponse))
        .catch(() => setShopifyConfigDetails(null))
    } else {
      setShopifyConfigDetails(null)
    }
  }, [providers, realmId])

  useEffect(() => {
    const wechatProvider = providers?.find((p) => p.platform === 'wechat')
    if (wechatProvider) {
      getWechatConfig({
        path: { realmId },
        query: { reveal_secrets: showWechatSecrets },
      })
        .then((result) => setWechatConfigDetails(result.data as WechatConfigResponse))
        .catch(() => setWechatConfigDetails(null))
    } else {
      setWechatConfigDetails(null)
    }
  }, [providers, realmId, showWechatSecrets])

  // Fetch Stripe and Creem config details from realm configs (shared API call)
  useEffect(() => {
    const stripeProvider = providers?.find((p) => p.platform === 'stripe')
    const creemProvider = providers?.find((p) => p.platform === 'creem')
    if (!stripeProvider && !creemProvider) {
      setStripeConfigDetails(null)
      setCreemConfigDetails(null)
      return
    }
    listRealmConfigs({ path: { realmId } })
      .then((result) => {
        const configs = result.data ?? []
        if (stripeProvider) {
          setStripeConfigDetails(parseStripeConfig(configs))
        } else {
          setStripeConfigDetails(null)
        }
        if (creemProvider) {
          setCreemConfigDetails({ enabled: parseCreemConfig(configs).enabled })
        } else {
          setCreemConfigDetails(null)
        }
      })
      .catch(() => {
        setStripeConfigDetails(null)
        setCreemConfigDetails(null)
      })
  }, [providers, realmId])

  const deleteMutation = useMutation({
    mutationFn: async () => {
      if (deleteProviderType === 'shopify') {
        return await deleteShopifyConfig({ path: { realmId } })
      } else if (deleteProviderType === 'wechat') {
        return await deleteWechatConfig({ path: { realmId } })
      } else {
        const configKeys =
          deleteProviderType === 'stripe'
            ? [{ configType: 'stripe', configKey: 'settings' }]
            : Object.values(CREEM_CONFIG_KEYS).map(
                (key) => ({ configType: 'creem', configKey: key })
              )
        await Promise.all(
          configKeys.map((k) => deleteRealmConfig({ path: { realmId, ...k } }))
        )
        return undefined
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      queryClient.invalidateQueries({ queryKey: ['realmConfig', realmId] })
      toast.success('Payment provider deleted successfully')
      setIsDeleteDialogOpen(false)
    },
    onError: (error: { status?: number; message?: string }) => {
      if (error?.status === 409) {
        toast.error('Cannot delete configuration while active subscriptions still exist.')
      } else {
        toast.error('Failed to delete configuration')
      }
    },
  })

  const handleNavigate = (type: 'shopify' | 'wechat' | 'stripe' | 'creem') => {
    void navigate({ to: `./${type}` })
  }

  const handleDelete = () => {
    deleteMutation.mutate()
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">Loading payment providers...</div>
      </div>
    )
  }

  const shopifyProvider = providers?.find((p) => p.platform === 'shopify')
  const wechatProvider = providers?.find((p) => p.platform === 'wechat')
  const stripeProvider = providers?.find((p) => p.platform === 'stripe')
  const creemProvider = providers?.find((p) => p.platform === 'creem')

  return (
    <div className="space-y-6" data-testid="payment-providers-page">
      <PageHeader title="Payment Providers" />

      {!shopifyProvider && !wechatProvider && !stripeProvider && !creemProvider ? (
        <Card className="border-dashed">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Plug2 className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold mb-2">No Payment Providers</h3>
            <p className="text-sm text-muted-foreground text-center mb-4">
              Configure a payment provider to start accepting subscriptions
            </p>
            <div className="flex gap-2 flex-wrap justify-center">
              <Button onClick={() => handleNavigate('shopify')} data-testid="add-shopify-button">
                <Plus className="mr-2 h-4 w-4" />
                Add Shopify
              </Button>
              <Button
                onClick={() => handleNavigate('wechat')}
                data-testid="add-wechat-button"
                variant="outline"
              >
                <Plus className="mr-2 h-4 w-4" />
                Add WeChat Pay
              </Button>
              <Button
                onClick={() => handleNavigate('stripe')}
                data-testid="add-stripe-button"
                variant="outline"
              >
                <Plus className="mr-2 h-4 w-4" />
                Add Stripe
              </Button>
              <Button
                onClick={() => handleNavigate('creem')}
                data-testid="add-creem-button"
                variant="outline"
              >
                <Plus className="mr-2 h-4 w-4" />
                Add Creem
              </Button>
            </div>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-4" data-testid="provider-list">
          {shopifyProvider && shopifyConfigDetails && (
            <ShopifyConfigDetail
              config={{
                shopDomain: shopifyConfigDetails.shopDomain || shopifyProvider.shopDomain || '',
                apiVersion: shopifyConfigDetails.apiVersion || shopifyProvider.apiVersion || '',
                webhookEndpoint: shopifyConfigDetails.webhookEndpoint,
                adminAccessToken: shopifyConfigDetails.adminAccessToken || '',
                storefrontAccessToken: shopifyConfigDetails.storefrontAccessToken || '',
                appClientSecret: shopifyConfigDetails.appClientSecret || '',
                lastUpdated: shopifyConfigDetails.lastUpdated || new Date().toISOString(),
                enabled: shopifyProvider.enabled,
              }}
              onEdit={() => handleNavigate('shopify')}
              onDelete={() => {
                setDeleteProviderType('shopify')
                setIsDeleteDialogOpen(true)
              }}
            />
          )}

          {wechatProvider && wechatConfigDetails && (
            <WechatConfigDetail
              config={wechatConfigDetails}
              onEdit={() => handleNavigate('wechat')}
              onDelete={() => {
                setDeleteProviderType('wechat')
                setIsDeleteDialogOpen(true)
              }}
              onShowSecrets={() => setShowWechatSecrets(true)}
              onHideSecrets={() => setShowWechatSecrets(false)}
              showSecrets={showWechatSecrets}
            />
          )}

          {stripeProvider && stripeConfigDetails && (
            <ProviderCard
              name="Stripe"
              description="Global payment platform"
              enabled={stripeConfigDetails.enabled}
              providerType="stripe"
              onEdit={handleNavigate}
              onDelete={() => {
                setDeleteProviderType('stripe')
                setIsDeleteDialogOpen(true)
              }}
            />
          )}

          {creemProvider && creemConfigDetails && (
            <ProviderCard
              name="Creem"
              description="Digital payment platform"
              enabled={creemConfigDetails.enabled}
              providerType="creem"
              onEdit={handleNavigate}
              onDelete={() => {
                setDeleteProviderType('creem')
                setIsDeleteDialogOpen(true)
              }}
            />
          )}

          {!shopifyProvider && (
            <Card className="border-dashed">
              <CardContent className="flex items-center justify-center py-6">
                <Button
                  onClick={() => handleNavigate('shopify')}
                  data-testid="add-shopify-button"
                  variant="outline"
                >
                  <Plus className="mr-2 h-4 w-4" />
                  Add Shopify Provider
                </Button>
              </CardContent>
            </Card>
          )}

          {!wechatProvider && (
            <Card className="border-dashed">
              <CardContent className="flex items-center justify-center py-6">
                <Button
                  onClick={() => handleNavigate('wechat')}
                  data-testid="add-wechat-button"
                  variant="outline"
                >
                  <Plus className="mr-2 h-4 w-4" />
                  Add WeChat Pay Provider
                </Button>
              </CardContent>
            </Card>
          )}

          {!stripeProvider && (
            <Card className="border-dashed">
              <CardContent className="flex items-center justify-center py-6">
                <Button
                  onClick={() => handleNavigate('stripe')}
                  data-testid="add-stripe-button"
                  variant="outline"
                >
                  <Plus className="mr-2 h-4 w-4" />
                  Add Stripe Provider
                </Button>
              </CardContent>
            </Card>
          )}

          {!creemProvider && (
            <Card className="border-dashed">
              <CardContent className="flex items-center justify-center py-6">
                <Button
                  onClick={() => handleNavigate('creem')}
                  data-testid="add-creem-button"
                  variant="outline"
                >
                  <Plus className="mr-2 h-4 w-4" />
                  Add Creem Provider
                </Button>
              </CardContent>
            </Card>
          )}
        </div>
      )}

      <DeleteConfirmDialog
        open={isDeleteDialogOpen}
        onOpenChange={setIsDeleteDialogOpen}
        onConfirm={handleDelete}
        configType={
          ({ shopify: 'Shopify', wechat: 'WeChat Pay', creem: 'Creem', stripe: 'Stripe' } as const)[
            deleteProviderType
          ]
        }
        activeSubscriptions={0}
        isDeleting={deleteMutation.isPending}
      />
    </div>
  )
}
