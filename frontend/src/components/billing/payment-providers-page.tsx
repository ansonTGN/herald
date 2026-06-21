import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
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
import { DeleteConfirmDialog } from './DeleteConfirmDialog'
import { Edit, Trash2, Plug2, Plus } from 'lucide-react'
import { listPaymentProviders } from '@/lib/api-generated'
import { deleteRealmConfig } from '@/lib/api-generated/sdk.gen'
import { STRIPE_CONFIG_KEYS } from '@/lib/billing-constants'
import { CREEM_CONFIG_KEYS } from '@/lib/creem-config-utils'
import { queryKeys } from '@/data/query-options'
import { m } from '@/paraglide/messages'

interface PaymentProvidersPageProps {
  realmId: string
}

export function PaymentProvidersPage({ realmId }: PaymentProvidersPageProps) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false)
  const [deleteProviderType, setDeleteProviderType] = useState<'stripe' | 'creem'>('stripe')

  const { data: providers, isLoading } = useQuery({
    queryKey: ['payment-providers', realmId],
    queryFn: async () => {
      const result = await listPaymentProviders({
        path: { realmId },
      })
      return result.data?.providers ?? []
    },
  })

  const deleteMutation = useMutation({
    mutationFn: async () => {
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
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      queryClient.invalidateQueries({ queryKey: ['realmConfig', realmId] })
      queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
      toast.success(m['billing.provider_deleted']())
      setIsDeleteDialogOpen(false)
    },
    onError: (error: { status?: number; message?: string }) => {
      if (error?.status === 409) {
        toast.error(m['billing.provider_delete_conflict']())
      } else {
        toast.error(m['billing.provider_delete_failed']())
      }
    },
  })

  const handleNavigate = (type: 'stripe' | 'creem') => {
    void navigate({ to: `./${type}` })
  }

  const handleDelete = () => {
    deleteMutation.mutate()
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">{m['billing.loading_providers']()}</div>
      </div>
    )
  }

  const stripeProvider = providers?.find((p) => p.platform === 'stripe')
  const creemProvider = providers?.find((p) => p.platform === 'creem')

  const hasAnyProvider = stripeProvider || creemProvider
  const unconfiguredProviders: {
    type: 'stripe' | 'creem'
    label: string
  }[] = []
  if (!stripeProvider) unconfiguredProviders.push({ type: 'stripe', label: 'Stripe' })
  if (!creemProvider) unconfiguredProviders.push({ type: 'creem', label: 'Creem' })

  return (
    <div className="space-y-6" data-testid="payment-providers-page">
      <PageHeader title={m['billing.payment_providers_title']()} />

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
              {m['billing.add_provider']({ name: label })}
            </Button>
          ))}
        </div>
      )}

      {hasAnyProvider ? (
        <Table data-testid="provider-list">
          <TableHeader>
            <TableRow>
              <TableHead>{m['billing.col_provider']()}</TableHead>
              <TableHead className="text-right">{m['common.actions']()}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {stripeProvider && (
              <TableRow data-testid="stripe-provider-row">
                <TableCell className="font-medium">Stripe</TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleNavigate('stripe')}
                      data-testid="edit-stripe-button"
                    >
                      <Edit className="mr-1 h-3 w-3" />
                      {m['common.edit']()}
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
                      {m['common.delete']()}
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            )}

            {creemProvider && (
              <TableRow data-testid="creem-provider-row">
                <TableCell className="font-medium">Creem</TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleNavigate('creem')}
                      data-testid="edit-creem-button"
                    >
                      <Edit className="mr-1 h-3 w-3" />
                      {m['common.edit']()}
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
                      {m['common.delete']()}
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
              {m['billing.no_providers_configured']()}
            </p>
          </CardContent>
        </Card>
      )}

      <DeleteConfirmDialog
        open={isDeleteDialogOpen}
        onOpenChange={setIsDeleteDialogOpen}
        onConfirm={handleDelete}
        configType={({ creem: 'Creem', stripe: 'Stripe' } as const)[deleteProviderType]}
        activeSubscriptions={0}
        isDeleting={deleteMutation.isPending}
      />
    </div>
  )
}
