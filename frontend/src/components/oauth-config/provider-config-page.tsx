import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Plus } from 'lucide-react'
import { ProviderList } from './provider-list'
import { ProviderConfigDialog } from './provider-config-dialog'
import type { OAuthConfigResponse } from '@/lib/api-generated'

interface ProviderConfigPageProps {
  realmId: string
}

export function ProviderConfigPage({ realmId }: ProviderConfigPageProps) {
  const [open, setOpen] = useState(false)
  const [editingConfig, setEditingConfig] = useState<OAuthConfigResponse | undefined>(undefined)

  const handleEdit = (config: OAuthConfigResponse) => {
    setEditingConfig(config)
    setOpen(true)
  }

  const handleAdd = () => {
    setEditingConfig(undefined)
    setOpen(true)
  }

  return (
    <div className="p-8 space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold">Identity Providers</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Configure OAuth providers for third-party login
          </p>
        </div>
        <Button onClick={handleAdd} data-testid="add-provider-button">
          <Plus className="mr-2 h-4 w-4" />
          Add Provider
        </Button>
      </div>

      {/* Provider List */}
      <div className="border rounded-lg p-6">
        <ProviderList realmId={realmId} onEdit={handleEdit} />
      </div>

      {/* Add/Edit Dialog */}
      <ProviderConfigDialog
        realmId={realmId}
        open={open}
        onOpenChange={setOpen}
        editingConfig={editingConfig}
      />
    </div>
  )
}
