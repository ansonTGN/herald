import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
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
    <>
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Identity Providers</CardTitle>
            <Button onClick={handleAdd} data-testid="add-provider-button">
              <Plus className="mr-2 h-4 w-4" />
              Add Provider
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <ProviderList realmId={realmId} onEdit={handleEdit} />
        </CardContent>
      </Card>

      <ProviderConfigDialog
        realmId={realmId}
        open={open}
        onOpenChange={setOpen}
        editingConfig={editingConfig}
      />
    </>
  )
}
