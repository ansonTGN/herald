import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Edit2, Trash2, ToggleLeft, ToggleRight } from 'lucide-react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Input } from '@/components/ui/input'
import { toast } from 'sonner'
import { paymentProviderMappingsQueryOptions } from '@/data/query-options'
import {
  createPaymentProviderMapping,
  updatePaymentProviderMapping,
  deletePaymentProviderMapping,
} from '@/lib/api-generated'
import type { PaymentProviderMappingResponse } from '@/lib/api-generated'
import { formatProviderName } from '@/components/billing/format-provider-name'

interface PaymentProviderConfigFormProps {
  packageId: string
  realmId: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function PaymentProviderConfigForm({
  packageId,
  realmId,
  open,
  onOpenChange,
}: PaymentProviderConfigFormProps) {
  const queryClient = useQueryClient()
  const [editingMapping, setEditingMapping] = useState<PaymentProviderMappingResponse | undefined>(
    undefined
  )
  const [formData, setFormData] = useState<{
    paymentProvider: 'wechat' | 'stripe' | 'creem' | ''
    externalProductId: string
    enabled: boolean
  }>({
    paymentProvider: '',
    externalProductId: '',
    enabled: true,
  })

  const { data: mappings, isLoading } = useQuery(
    paymentProviderMappingsQueryOptions(realmId, packageId)
  )

  const createMutation = useMutation({
    mutationFn: async (data: typeof formData) => {
      const response = await createPaymentProviderMapping({
        path: { realmId, packageId },
        body: {
          paymentProvider: data.paymentProvider || 'stripe',
          externalProductId: data.externalProductId || null,
          enabled: data.enabled,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Payment provider mapping added')
      setEditingMapping(undefined)
      setFormData({ paymentProvider: '', externalProductId: '', enabled: true })
      queryClient.invalidateQueries({ queryKey: ['payment-provider-mappings', realmId, packageId] })
    },
    onError: (error: Error) => {
      toast.error(`Failed to add mapping: ${error.message}`)
    },
  })

  const updateMutation = useMutation({
    mutationFn: async (data: {
      mapping: PaymentProviderMappingResponse
      updates: typeof formData
    }) => {
      const response = await updatePaymentProviderMapping({
        path: { realmId, packageId, mappingId: data.mapping.id },
        body: {
          externalProductId: data.updates.externalProductId || null,
          enabled: data.updates.enabled,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Payment provider mapping updated')
      setEditingMapping(undefined)
      setFormData({ paymentProvider: '', externalProductId: '', enabled: true })
      queryClient.invalidateQueries({ queryKey: ['payment-provider-mappings', realmId, packageId] })
    },
    onError: (error: Error) => {
      toast.error(`Failed to update mapping: ${error.message}`)
    },
  })

  const deleteMutation = useMutation({
    mutationFn: async (mappingId: string) => {
      const response = await deletePaymentProviderMapping({
        path: { realmId, packageId, mappingId },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Payment provider mapping removed')
      queryClient.invalidateQueries({ queryKey: ['payment-provider-mappings', realmId, packageId] })
    },
    onError: (error: Error) => {
      toast.error(`Failed to remove mapping: ${error.message}`)
    },
  })

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    if (editingMapping) {
      await updateMutation.mutateAsync({ mapping: editingMapping, updates: formData })
    } else {
      await createMutation.mutateAsync(formData)
    }
  }

  const handleEdit = (mapping: PaymentProviderMappingResponse) => {
    setEditingMapping(mapping)
    setFormData({
      paymentProvider: mapping.paymentProvider as 'wechat' | 'stripe' | 'creem',
      externalProductId: mapping.externalProductId || '',
      enabled: mapping.enabled,
    })
  }

  const handleDelete = async (mappingId: string) => {
    await deleteMutation.mutateAsync(mappingId)
  }

  const handleToggle = async (mapping: PaymentProviderMappingResponse) => {
    await updateMutation.mutateAsync({
      mapping,
      updates: {
        paymentProvider: mapping.paymentProvider as 'wechat' | 'stripe' | 'creem',
        externalProductId: mapping.externalProductId || '',
        enabled: !mapping.enabled,
      },
    })
  }

  const availableProviders = ['wechat', 'stripe', 'creem']
  const usedProviders = mappings?.map((m) => m.paymentProvider) || []
  const unusedProviders = availableProviders.filter((p) => !usedProviders.includes(p))

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl" data-testid="payment-provider-config-dialog">
        <DialogHeader>
          <DialogTitle>Configure Payment Providers</DialogTitle>
          <DialogDescription>
            Configure which payment providers can be used to purchase this points package
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4" data-testid="payment-provider-list">
          {isLoading ? (
            <div className="py-8 text-center text-sm text-muted-foreground">Loading...</div>
          ) : mappings && mappings.length > 0 ? (
            <div className="space-y-3">
              {mappings.map((mapping) => (
                <div
                  key={mapping.id}
                  className="flex items-center justify-between rounded-lg border p-4"
                >
                  <div className="flex items-center gap-4">
                    <Badge variant={mapping.enabled ? 'default' : 'secondary'}>
                      {formatProviderName(mapping.paymentProvider)}
                    </Badge>
                    <div className="text-sm">
                      <div className="font-medium">External Product ID</div>
                      <div className="text-muted-foreground">
                        {mapping.externalProductId || 'Not configured'}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleToggle(mapping)}
                      data-testid={`payment-provider-enabled-switch-${mapping.paymentProvider}`}
                    >
                      {mapping.enabled ? (
                        <ToggleRight className="h-4 w-4 text-green-600" />
                      ) : (
                        <ToggleLeft className="h-4 w-4 text-muted-foreground" />
                      )}
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleEdit(mapping)}
                      data-testid={`payment-provider-edit-button-${mapping.paymentProvider}`}
                    >
                      <Edit2 className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleDelete(mapping.id)}
                      data-testid={`payment-provider-remove-button-${mapping.paymentProvider}`}
                    >
                      <Trash2 className="h-4 w-4 text-destructive" />
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="py-8 text-center text-sm text-muted-foreground">
              No payment providers configured yet
            </div>
          )}

          {editingMapping || !editingMapping ? (
            <form onSubmit={handleSubmit} className="space-y-4 border-t pt-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="paymentProvider">Payment Provider</Label>
                  <select
                    id="paymentProvider"
                    className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-sm shadow-sm transition-colors"
                    value={formData.paymentProvider}
                    onChange={(e) =>
                      setFormData({
                        ...formData,
                        paymentProvider: e.target.value as 'wechat' | 'stripe' | 'creem',
                      })
                    }
                    disabled={!!editingMapping}
                    required
                  >
                    <option value="">Select a provider</option>
                    {(editingMapping ? [editingMapping.paymentProvider] : unusedProviders).map(
                      (provider) => (
                        <option key={provider} value={provider}>
                          {formatProviderName(provider)}
                        </option>
                      )
                    )}
                  </select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="externalProductId">External Product ID</Label>
                  <Input
                    id="externalProductId"
                    value={formData.externalProductId}
                    onChange={(e) =>
                      setFormData({ ...formData, externalProductId: e.target.value })
                    }
                    placeholder="Optional external product ID"
                    data-testid={`payment-provider-external-id-input-${formData.paymentProvider}`}
                  />
                </div>
              </div>

              <div className="flex items-center space-x-2">
                <Switch
                  id="enabled"
                  checked={formData.enabled}
                  onCheckedChange={(checked) => setFormData({ ...formData, enabled: checked })}
                  data-testid={`payment-provider-enabled-switch-${formData.paymentProvider}`}
                />
                <Label htmlFor="enabled">Enabled</Label>
              </div>

              <DialogFooter>
                {editingMapping && (
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => {
                      setEditingMapping(undefined)
                      setFormData({ paymentProvider: '', externalProductId: '', enabled: true })
                    }}
                  >
                    Cancel
                  </Button>
                )}
                <Button
                  type="submit"
                  disabled={
                    createMutation.isPending ||
                    updateMutation.isPending ||
                    !formData.paymentProvider
                  }
                  data-testid="payment-provider-add-button"
                >
                  {editingMapping ? 'Update' : 'Add'} Payment Provider
                </Button>
              </DialogFooter>
            </form>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  )
}
