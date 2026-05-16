import { useState, useEffect } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { PageHeader } from '@/components/shared/page-header'
import { ShopifyConfigFormDialog } from './ShopifyConfigForm'
import { ShopifyConfigDetail } from './ShopifyConfigDetail'
import { WechatConfigFormDialog } from './WechatConfigForm'
import { StripeConfigFormDialog } from './StripeConfigForm'
import { WechatConfigDetail } from './WechatConfigDetail'
import { DeleteConfirmDialog } from './DeleteConfirmDialog'
import { TestConnectionDialog } from './TestConnectionDialog'
import { Edit, Trash2, Plug2, Plus } from 'lucide-react'
import {
  getShopifyConfigDefaults,
  getWechatConfigDefaults,
  type ShopifyConfigForm,
  type WechatConfigForm,
} from '@/lib/schemas/billing-forms'
import {
  getStripeConfigDefaults,
  type StripeConfigForm as StripeConfigFormValues,
} from '@/lib/schemas/stripe-config'
import {
  createShopifyConfig,
  deleteShopifyConfig,
  getShopifyConfig,
  listPaymentProviders,
  testShopifyConnection,
  updateShopifyConfig,
  createWechatConfig,
  deleteWechatConfig,
  getWechatConfig,
  updateWechatConfig,
  type PaymentProviderInfo,
  type ShopifyConfigResponse,
  type WechatConfigResponse,
  type TestConnectionResponse,
} from '@/lib/api-generated'
import {
  listRealmConfigs,
  batchUpsertRealmConfigs,
  deleteRealmConfig,
} from '@/lib/api-generated/sdk.gen'
import { parseStripeConfig, buildStripeConfigRequest } from '@/lib/stripe-config-utils'

interface PaymentProvidersPageProps {
  realmId: string
}

export function PaymentProvidersPage({ realmId }: PaymentProvidersPageProps) {
  const queryClient = useQueryClient()
  const [isFormOpen, setIsFormOpen] = useState(false)
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false)
  const [isTestDialogOpen, setIsTestDialogOpen] = useState(false)
  const [formMode, setFormMode] = useState<'create' | 'edit'>('create')
  const [currentConfig, setCurrentConfig] = useState<
    Partial<ShopifyConfigForm | WechatConfigForm | StripeConfigFormValues>
  >({})
  const [testConfig, setTestConfig] = useState<
    ShopifyConfigForm | WechatConfigForm | StripeConfigFormValues | null
  >(null)
  const [isLoadingConfig, setIsLoadingConfig] = useState(false)
  const [configType, setConfigType] = useState<'shopify' | 'wechat' | 'stripe'>('shopify')
  const [shopifyConfigDetails, setShopifyConfigDetails] = useState<ShopifyConfigResponse | null>(
    null
  )
  const [wechatConfigDetails, setWechatConfigDetails] = useState<WechatConfigResponse | null>(null)
  const [stripeConfigDetails, setStripeConfigDetails] = useState<StripeConfigFormValues | null>(
    null
  )
  const [showWechatSecrets, setShowWechatSecrets] = useState(false)

  // Auto-hide WeChat secrets after 5 seconds
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

  // Query for payment providers
  const { data: providers, isLoading } = useQuery({
    queryKey: ['payment-providers', realmId],
    queryFn: async () => {
      const result = await listPaymentProviders({
        path: { realmId },
      })
      return result.data?.providers ?? []
    },
  })

  // Fetch Shopify config details when Shopify provider exists
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

  // Fetch WeChat config details when WeChat provider exists
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

  // Fetch Stripe config details when Stripe provider exists
  useEffect(() => {
    const stripeProvider = providers?.find((p) => p.platform === 'stripe')
    if (stripeProvider) {
      listRealmConfigs({ path: { realmId } })
        .then((result) => {
          const stripeConfig = parseStripeConfig(result.data ?? [])
          setStripeConfigDetails(stripeConfig)
        })
        .catch(() => setStripeConfigDetails(null))
    } else {
      setStripeConfigDetails(null)
    }
  }, [providers, realmId])

  // Mutation factory to reduce duplication (must be a custom hook to use useMutation)
  function useConfigMutation<
    T extends ShopifyConfigForm | WechatConfigForm | StripeConfigFormValues,
  >(
    mutationFn: (config: T) => Promise<unknown>,
    successMessage: string,
    errorMessagePrefix: string
  ) {
    return useMutation({
      mutationFn,
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
        queryClient.invalidateQueries({ queryKey: ['realmConfig', realmId] })
        toast.success(successMessage)
        setIsFormOpen(false)
      },
      onError: (error: { status?: number; message?: string }) => {
        if (error?.status === 409) {
          const providerName =
            configType === 'shopify' ? 'Shopify' : configType === 'wechat' ? 'WeChat Pay' : 'Stripe'
          toast.error(
            `A ${providerName} configuration already exists. Please edit the existing one.`
          )
        } else if (error?.status === 422) {
          toast.error('Connection test failed. Please check your credentials.')
        } else {
          toast.error(`${errorMessagePrefix}: ${error?.message || 'Unknown error'}`)
        }
      },
    })
  }

  // Create mutation
  const createMutation = useConfigMutation(
    async (config: ShopifyConfigForm | WechatConfigForm | StripeConfigFormValues) => {
      if (configType === 'shopify') {
        const shopifyConfig = config as ShopifyConfigForm
        return createShopifyConfig({
          path: { realmId },
          body: {
            shopDomain: shopifyConfig.shopDomain,
            adminAccessToken: shopifyConfig.adminAccessToken,
            storefrontAccessToken: shopifyConfig.storefrontAccessToken,
            appClientSecret: shopifyConfig.appClientSecret,
            apiVersion: shopifyConfig.apiVersion,
            webhookSubscriptionMode: shopifyConfig.webhookSubscriptionMode as
              | 'admin_api'
              | 'event_bridge',
            timeout: shopifyConfig.timeout,
            skipConnectionTest: shopifyConfig.skipConnectionTest,
          },
        })
      } else if (configType === 'wechat') {
        const wechatConfig = config as WechatConfigForm
        return createWechatConfig({
          path: { realmId },
          body: {
            appId: wechatConfig.appId,
            mchId: wechatConfig.mchId,
            privateKey: wechatConfig.privateKey,
            serialNo: wechatConfig.serialNo,
            v3Key: wechatConfig.v3Key,
            notifyUrl: wechatConfig.notifyUrl,
          },
        })
      } else {
        const stripeConfig = config as StripeConfigFormValues
        return batchUpsertRealmConfigs({
          path: { realmId },
          body: { configs: [buildStripeConfigRequest(stripeConfig)] },
        })
      }
    },
    'Configuration created successfully',
    'Failed to create configuration'
  )

  // Update mutation
  const updateMutation = useConfigMutation(
    async (config: ShopifyConfigForm | WechatConfigForm | StripeConfigFormValues) => {
      if (configType === 'shopify') {
        const shopifyConfig = config as ShopifyConfigForm
        return updateShopifyConfig({
          path: { realmId },
          body: {
            shopDomain: shopifyConfig.shopDomain,
            adminAccessToken: shopifyConfig.adminAccessToken,
            storefrontAccessToken: shopifyConfig.storefrontAccessToken,
            appClientSecret: shopifyConfig.appClientSecret,
            apiVersion: shopifyConfig.apiVersion,
            webhookSubscriptionMode: shopifyConfig.webhookSubscriptionMode as
              | 'admin_api'
              | 'event_bridge',
            timeout: shopifyConfig.timeout,
            skipConnectionTest: shopifyConfig.skipConnectionTest,
          },
        })
      } else if (configType === 'wechat') {
        const wechatConfig = config as WechatConfigForm
        return updateWechatConfig({
          path: { realmId },
          body: {
            appId: wechatConfig.appId,
            mchId: wechatConfig.mchId,
            privateKey: wechatConfig.privateKey,
            serialNo: wechatConfig.serialNo,
            v3Key: wechatConfig.v3Key,
            notifyUrl: wechatConfig.notifyUrl,
          },
        })
      } else {
        const stripeConfig = config as StripeConfigFormValues
        return batchUpsertRealmConfigs({
          path: { realmId },
          body: { configs: [buildStripeConfigRequest(stripeConfig)] },
        })
      }
    },
    'Configuration updated successfully',
    'Failed to update configuration'
  )

  // Delete mutation
  const deleteMutation = useMutation({
    mutationFn: async () => {
      if (configType === 'shopify') {
        return await deleteShopifyConfig({ path: { realmId } })
      } else if (configType === 'wechat') {
        return await deleteWechatConfig({ path: { realmId } })
      } else {
        // For Stripe, we need to delete all realm config entries with config_type 'stripe'
        const result = await listRealmConfigs({ path: { realmId } })
        const stripeConfigs = (result.data ?? []).filter((c) => c.configType === 'stripe')
        await Promise.all(
          stripeConfigs.map((config) =>
            deleteRealmConfig({
              path: { realmId, configType: config.configType, configKey: config.configKey },
            })
          )
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

  // Test connection mutation
  const testMutation = useMutation({
    mutationFn: (config: ShopifyConfigForm | WechatConfigForm) => {
      if (configType === 'shopify') {
        return testShopifyConnection({
          path: { realmId },
          body: {
            shopDomain: (config as ShopifyConfigForm).shopDomain,
            adminAccessToken: (config as ShopifyConfigForm).adminAccessToken,
            storefrontAccessToken: (config as ShopifyConfigForm).storefrontAccessToken,
          },
        })
      } else {
        // WeChat Pay doesn't have a test connection endpoint
        // Return a mock success response
        return Promise.resolve({
          data: {
            success: true,
            results: {
              success: 'success',
            },
          },
        } as unknown as ReturnType<typeof testShopifyConnection>)
      }
    },
    onError: (error: { message?: string }) => {
      toast.error(`Connection test failed: ${error?.message || 'Unknown error'}`)
    },
  })

  const handleCreate = (type: 'shopify' | 'wechat' | 'stripe' = 'shopify') => {
    setFormMode('create')
    setConfigType(type)
    if (type === 'shopify') {
      setCurrentConfig(getShopifyConfigDefaults())
    } else if (type === 'wechat') {
      setCurrentConfig(getWechatConfigDefaults())
    } else {
      setCurrentConfig(getStripeConfigDefaults())
    }
    setIsFormOpen(true)
  }

  const handleEdit = async (provider: PaymentProviderInfo) => {
    setFormMode('edit')
    setConfigType(
      provider.platform === 'shopify'
        ? 'shopify'
        : provider.platform === 'wechat'
          ? 'wechat'
          : 'stripe'
    )
    setIsLoadingConfig(true)

    try {
      if (provider.platform === 'shopify') {
        const result = await getShopifyConfig({
          path: { realmId },
        })
        const config = result.data as ShopifyConfigResponse | undefined
        if (!config) {
          toast.error('Failed to load Shopify configuration')
          return
        }
        setCurrentConfig({
          shopDomain: config.shopDomain || provider.shopDomain || '',
          adminAccessToken: '',
          storefrontAccessToken: '',
          appClientSecret: '',
          apiVersion: config.apiVersion,
          webhookSubscriptionMode: config.webhookSubscriptionMode as 'admin_api' | 'event_bridge',
          timeout: config.timeout,
        })
      } else if (provider.platform === 'wechat') {
        const result = await getWechatConfig({
          path: { realmId },
        })
        const config = result.data as WechatConfigResponse | undefined
        if (!config) {
          toast.error('Failed to load WeChat configuration')
          return
        }
        setCurrentConfig({
          appId: config.appId || '',
          mchId: config.mchId || '',
          privateKey: '',
          serialNo: config.serialNo || '',
          v3Key: '',
          notifyUrl: config.notifyUrl || '',
        })
      } else {
        // Stripe
        const result = await listRealmConfigs({ path: { realmId } })
        const stripeConfig = parseStripeConfig(result.data ?? [])
        setCurrentConfig({
          enabled: stripeConfig.enabled,
          publishableKey: stripeConfig.publishableKey,
          secretKey: '',
          webhookSecret: '',
        })
      }
      setIsFormOpen(true)
    } catch {
      toast.error(
        `Failed to load ${provider.platform === 'shopify' ? 'Shopify' : provider.platform === 'wechat' ? 'WeChat' : 'Stripe'} configuration`
      )
    } finally {
      setIsLoadingConfig(false)
    }
  }

  const handleDelete = () => {
    deleteMutation.mutate()
  }

  const handleTestConnection = (config: ShopifyConfigForm | WechatConfigForm) => {
    setTestConfig(config)
    setIsTestDialogOpen(true)
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

  return (
    <div className="space-y-6" data-testid="payment-providers-page">
      <PageHeader title="Payment Providers" />

      {!shopifyProvider && !wechatProvider && !stripeProvider ? (
        <Card className="border-dashed">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <Plug2 className="h-12 w-12 text-muted-foreground mb-4" />
            <h3 className="text-lg font-semibold mb-2">No Payment Providers</h3>
            <p className="text-sm text-muted-foreground text-center mb-4">
              Configure a payment provider to start accepting subscriptions
            </p>
            <div className="flex gap-2 flex-wrap justify-center">
              <Button onClick={() => handleCreate('shopify')} data-testid="add-shopify-button">
                <Plus className="mr-2 h-4 w-4" />
                Add Shopify
              </Button>
              <Button
                onClick={() => handleCreate('wechat')}
                data-testid="add-wechat-button"
                variant="outline"
              >
                <Plus className="mr-2 h-4 w-4" />
                Add WeChat Pay
              </Button>
              <Button
                onClick={() => handleCreate('stripe')}
                data-testid="add-stripe-button"
                variant="outline"
              >
                <Plus className="mr-2 h-4 w-4" />
                Add Stripe
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
              onEdit={() => void handleEdit(shopifyProvider)}
              onDelete={() => {
                setConfigType('shopify')
                setIsDeleteDialogOpen(true)
              }}
            />
          )}

          {wechatProvider && wechatConfigDetails && (
            <WechatConfigDetail
              config={wechatConfigDetails}
              onEdit={() => void handleEdit(wechatProvider)}
              onDelete={() => {
                setConfigType('wechat')
                setIsDeleteDialogOpen(true)
              }}
              onShowSecrets={() => setShowWechatSecrets(true)}
              onHideSecrets={() => setShowWechatSecrets(false)}
              showSecrets={showWechatSecrets}
            />
          )}

          {stripeProvider && stripeConfigDetails && (
            <Card>
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div>
                    <CardTitle>Stripe</CardTitle>
                    <CardDescription>
                      Global payment platform ·{' '}
                      {stripeConfigDetails.enabled ? 'Enabled' : 'Disabled'}
                    </CardDescription>
                  </div>
                  <Badge variant={stripeConfigDetails.enabled ? 'default' : 'secondary'}>
                    {stripeConfigDetails.enabled ? 'Active' : 'Disabled'}
                  </Badge>
                </div>
              </CardHeader>
              <CardContent>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => void handleEdit(stripeProvider)}
                    disabled={isLoadingConfig}
                    data-testid="edit-stripe-button"
                  >
                    <Edit className="mr-2 h-4 w-4" />
                    {isLoadingConfig ? 'Loading...' : 'Edit'}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      setConfigType('stripe')
                      setIsDeleteDialogOpen(true)
                    }}
                    data-testid="delete-stripe-button"
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    Delete
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          {/* Add buttons for missing providers */}
          {!shopifyProvider && (
            <Card className="border-dashed">
              <CardContent className="flex items-center justify-center py-6">
                <Button
                  onClick={() => handleCreate('shopify')}
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
                  onClick={() => handleCreate('wechat')}
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
                  onClick={() => handleCreate('stripe')}
                  data-testid="add-stripe-button"
                  variant="outline"
                >
                  <Plus className="mr-2 h-4 w-4" />
                  Add Stripe Provider
                </Button>
              </CardContent>
            </Card>
          )}
        </div>
      )}

      <ShopifyConfigFormDialog
        open={isFormOpen && configType === 'shopify'}
        onOpenChange={setIsFormOpen}
        initialValues={currentConfig as Partial<ShopifyConfigForm>}
        onSubmit={(data) => {
          if (formMode === 'create') {
            createMutation.mutate(data)
          } else {
            updateMutation.mutate(data)
          }
        }}
        onTestConnection={(config) => {
          handleTestConnection(config)
        }}
        isSubmitting={createMutation.isPending || updateMutation.isPending}
        isTesting={testMutation.isPending}
        mode={formMode}
      />

      <WechatConfigFormDialog
        open={isFormOpen && configType === 'wechat'}
        onOpenChange={setIsFormOpen}
        initialValues={currentConfig as Partial<WechatConfigForm>}
        onSubmit={(data) => {
          if (formMode === 'create') {
            createMutation.mutate(data)
          } else {
            updateMutation.mutate(data)
          }
        }}
        isSubmitting={createMutation.isPending || updateMutation.isPending}
        mode={formMode}
      />

      <StripeConfigFormDialog
        open={isFormOpen && configType === 'stripe'}
        onOpenChange={setIsFormOpen}
        initialValues={currentConfig as Partial<StripeConfigFormValues>}
        onSubmit={async (data) => {
          if (formMode === 'create') {
            await createMutation.mutateAsync(data)
          } else {
            await updateMutation.mutateAsync(data)
          }
        }}
        mode={formMode}
      />

      <DeleteConfirmDialog
        open={isDeleteDialogOpen}
        onOpenChange={setIsDeleteDialogOpen}
        onConfirm={handleDelete}
        configType={
          configType === 'shopify' ? 'Shopify' : configType === 'wechat' ? 'WeChat Pay' : 'Stripe'
        }
        activeSubscriptions={0}
        isDeleting={deleteMutation.isPending}
      />

      {testConfig && configType === 'shopify' && (
        <TestConnectionDialog
          open={isTestDialogOpen}
          onOpenChange={setIsTestDialogOpen}
          config={testConfig as ShopifyConfigForm}
          onTest={async (config) => {
            try {
              const result = await testMutation.mutateAsync(config)
              const data = result.data as TestConnectionResponse | undefined

              if (!data) {
                return {
                  adminApiSuccess: false,
                  adminApiMessage: 'No response data received',
                  storefrontApiSuccess: false,
                  storefrontApiMessage: 'No response data received',
                  shopAccessSuccess: false,
                  shopAccessMessage: 'No response data received',
                }
              }

              // Extract error messages if they exist
              const errorMessages = data.errors?.join('\n') || 'Connection test failed'

              return {
                adminApiSuccess: data?.results?.adminApiConnection === 'success',
                adminApiMessage:
                  data?.results?.adminApiConnection === 'success'
                    ? 'Admin API connection successful'
                    : data?.results?.adminApiConnection || errorMessages,
                storefrontApiSuccess: data?.results?.storefrontApiConnection === 'success',
                storefrontApiMessage:
                  data?.results?.storefrontApiConnection === 'success'
                    ? 'Storefront API connection successful'
                    : data?.results?.storefrontApiConnection || errorMessages,
                shopAccessSuccess: data?.results?.shopAccess === 'success',
                shopAccessMessage:
                  data?.results?.shopAccess === 'success'
                    ? 'Shop access verified'
                    : data?.results?.shopAccess || errorMessages,
              }
            } catch (error) {
              // Handle any errors from the mutation itself
              const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred'
              return {
                adminApiSuccess: false,
                adminApiMessage: errorMessage,
                storefrontApiSuccess: false,
                storefrontApiMessage: errorMessage,
                shopAccessSuccess: false,
                shopAccessMessage: errorMessage,
              }
            }
          }}
        />
      )}
    </div>
  )
}
