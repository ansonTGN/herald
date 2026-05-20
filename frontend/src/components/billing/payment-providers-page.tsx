import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { PageHeader } from '@/components/shared/page-header'
import { ShopifyConfigFields } from './ShopifyConfigDetail'
import { DeleteConfirmDialog } from './DeleteConfirmDialog'
import { Edit, Trash2, Plug2, Plus, ChevronDown, ChevronUp } from 'lucide-react'
import {
  deleteShopifyConfig,
  getShopifyConfig,
  listPaymentProviders,
  deleteWechatConfig,
  type ShopifyConfigResponse,
} from '@/lib/api-generated'
import { listRealmConfigs, deleteRealmConfig } from '@/lib/api-generated/sdk.gen'
import { parseStripeConfig } from '@/lib/stripe-config-utils'
import { STRIPE_CONFIG_KEYS } from '@/lib/billing-constants'
import { parseCreemConfig, CREEM_CONFIG_KEYS } from '@/lib/creem-config-utils'
import { queryKeys } from '@/data/query-options'

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
  const [stripeConfigDetails, setStripeConfigDetails] = useState<{ enabled: boolean } | null>(null)
  const [creemConfigDetails, setCreemConfigDetails] = useState<{ enabled: boolean } | null>(null)
  const [showShopifySecrets, setShowShopifySecrets] = useState(false)
  const [expandedProvider, setExpandedProvider] = useState<'shopify' | null>(null)

  // Auto-hide Shopify secrets after 5 seconds
  useEffect(() => {
    let timeoutId: ReturnType<typeof setTimeout>
    if (showShopifySecrets) {
      timeoutId = setTimeout(() => {
        setShowShopifySecrets(false)
      }, 5000)
    }
    return () => {
      if (timeoutId) clearTimeout(timeoutId)
    }
  }, [showShopifySecrets])

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
    }
  }, [providers, realmId])

  // Fetch Stripe and Creem config details from realm configs (shared API call)
  useEffect(() => {
    const stripeProvider = providers?.find((p) => p.platform === 'stripe')
    const creemProvider = providers?.find((p) => p.platform === 'creem')
    if (stripeProvider || creemProvider) {
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
    }
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
            ? Object.values(STRIPE_CONFIG_KEYS).map((key) => ({
                configType: 'stripe',
                configKey: key,
              }))
            : Object.values(CREEM_CONFIG_KEYS).map((key) => ({
                configType: 'creem',
                configKey: key,
              }))
        // Delete all keys, ignoring 404s for keys that don't exist
        await Promise.all(
          configKeys.map((k) =>
            deleteRealmConfig({ path: { realmId, ...k } }).catch((e) => {
              if (e?.status !== 404) throw e
            })
          )
        )
        return undefined
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      queryClient.invalidateQueries({ queryKey: ['realmConfig', realmId] })
      queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
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

  const toggleExpand = () => {
    setExpandedProvider((prev) => (prev === 'shopify' ? null : 'shopify'))
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

  const hasAnyProvider = shopifyProvider || wechatProvider || stripeProvider || creemProvider
  const unconfiguredProviders: {
    type: 'shopify' | 'wechat' | 'stripe' | 'creem'
    label: string
  }[] = []
  if (!shopifyProvider) unconfiguredProviders.push({ type: 'shopify', label: 'Shopify' })
  if (!wechatProvider) unconfiguredProviders.push({ type: 'wechat', label: 'WeChat Pay' })
  if (!stripeProvider) unconfiguredProviders.push({ type: 'stripe', label: 'Stripe' })
  if (!creemProvider) unconfiguredProviders.push({ type: 'creem', label: 'Creem' })

  return (
    <div className="space-y-6" data-testid="payment-providers-page">
      <PageHeader title="Payment Providers" />

      {unconfiguredProviders.length > 0 && (
        <div className="flex gap-2 flex-wrap">
          {unconfiguredProviders.map(({ type, label }) => (
            <Button
              key={type}
              onClick={() => handleNavigate(type)}
              data-testid={`add-${type}-button`}
              variant="outline"
            >
              <Plus className="mr-2 h-4 w-4" />
              Add {label}
            </Button>
          ))}
        </div>
      )}

      {hasAnyProvider ? (
        <Table data-testid="provider-list">
          <TableHeader>
            <TableRow>
              <TableHead>Provider</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {shopifyProvider && shopifyConfigDetails && (
              <>
                <TableRow data-testid="shopify-provider-row">
                  <TableCell className="font-medium">Shopify</TableCell>
                  <TableCell>
                    <Badge variant={shopifyProvider.enabled ? 'default' : 'secondary'}>
                      {shopifyProvider.enabled ? 'Active' : 'Disabled'}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-2">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => toggleExpand()}
                        data-testid="toggle-shopify-details-button"
                      >
                        {expandedProvider === 'shopify' ? (
                          <ChevronUp className="h-4 w-4" />
                        ) : (
                          <ChevronDown className="h-4 w-4" />
                        )}
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleNavigate('shopify')}
                        data-testid="edit-shopify-button"
                      >
                        <Edit className="mr-1 h-3 w-3" />
                        Edit
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          setDeleteProviderType('shopify')
                          setIsDeleteDialogOpen(true)
                        }}
                        data-testid="delete-shopify-button"
                      >
                        <Trash2 className="mr-1 h-3 w-3" />
                        Delete
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
                {expandedProvider === 'shopify' && (
                  <TableRow data-testid="shopify-details-row">
                    <TableCell colSpan={3} className="p-0">
                      <ShopifyConfigFields
                        config={{
                          shopDomain:
                            shopifyConfigDetails.shopDomain || shopifyProvider.shopDomain || '',
                          apiVersion:
                            shopifyConfigDetails.apiVersion || shopifyProvider.apiVersion || '',
                          webhookEndpoint: shopifyConfigDetails.webhookEndpoint,
                          adminAccessToken: shopifyConfigDetails.adminAccessToken || '',
                          storefrontAccessToken: shopifyConfigDetails.storefrontAccessToken || '',
                          appClientSecret: shopifyConfigDetails.appClientSecret || '',
                          lastUpdated: shopifyConfigDetails.lastUpdated || new Date().toISOString(),
                          enabled: shopifyProvider.enabled,
                        }}
                        showSecrets={showShopifySecrets}
                        onShowSecrets={() => setShowShopifySecrets(true)}
                        onHideSecrets={() => setShowShopifySecrets(false)}
                      />
                    </TableCell>
                  </TableRow>
                )}
              </>
            )}

            {wechatProvider && (
              <TableRow data-testid="wechat-provider-row">
                <TableCell className="font-medium">WeChat Pay</TableCell>
                <TableCell>
                  <Badge variant={wechatProvider.enabled ? 'default' : 'secondary'}>
                    {wechatProvider.enabled ? 'Active' : 'Disabled'}
                  </Badge>
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleNavigate('wechat')}
                      data-testid="edit-wechat-button"
                    >
                      <Edit className="mr-1 h-3 w-3" />
                      Edit
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        setDeleteProviderType('wechat')
                        setIsDeleteDialogOpen(true)
                      }}
                      data-testid="delete-wechat-button"
                    >
                      <Trash2 className="mr-1 h-3 w-3" />
                      Delete
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            )}

            {stripeProvider && stripeConfigDetails && (
              <TableRow data-testid="stripe-provider-row">
                <TableCell className="font-medium">Stripe</TableCell>
                <TableCell>
                  <Badge variant={stripeConfigDetails.enabled ? 'default' : 'secondary'}>
                    {stripeConfigDetails.enabled ? 'Active' : 'Disabled'}
                  </Badge>
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleNavigate('stripe')}
                      data-testid="edit-stripe-button"
                    >
                      <Edit className="mr-1 h-3 w-3" />
                      Edit
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        setDeleteProviderType('stripe')
                        setIsDeleteDialogOpen(true)
                      }}
                      data-testid="delete-stripe-button"
                    >
                      <Trash2 className="mr-1 h-3 w-3" />
                      Delete
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            )}

            {creemProvider && creemConfigDetails && (
              <TableRow data-testid="creem-provider-row">
                <TableCell className="font-medium">Creem</TableCell>
                <TableCell>
                  <Badge variant={creemConfigDetails.enabled ? 'default' : 'secondary'}>
                    {creemConfigDetails.enabled ? 'Active' : 'Disabled'}
                  </Badge>
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleNavigate('creem')}
                      data-testid="edit-creem-button"
                    >
                      <Edit className="mr-1 h-3 w-3" />
                      Edit
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        setDeleteProviderType('creem')
                        setIsDeleteDialogOpen(true)
                      }}
                      data-testid="delete-creem-button"
                    >
                      <Trash2 className="mr-1 h-3 w-3" />
                      Delete
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      ) : (
        <Card className="border-dashed">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Plug2 className="h-12 w-12 text-muted-foreground mb-4" />
            <p className="text-sm text-muted-foreground text-center">
              No providers configured yet. Click a button above to get started.
            </p>
          </CardContent>
        </Card>
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
