import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { ChangePasswordForm } from '@/components/profile/change-password-form'
import { TotpStatusCard } from '@/components/profile/totp/totp-status-card'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { TotpDisableForm } from '@/components/profile/totp/totp-disable-form'
import { TotpRegenerateForm } from '@/components/profile/totp/totp-regenerate-form'
import { PageHeader } from '@/components/shared'

export const Route = createFileRoute('/$realmId/user/security/')({
  component: ProfileSecurity,
})

type TotpDialogType = 'disable' | 'regenerate' | null

function ProfileSecurity() {
  const navigate = useNavigate()
  const { realmId } = Route.useParams()
  const [totpDialog, setTotpDialog] = useState<TotpDialogType>(null)

  const handleDialogClose = () => setTotpDialog(null)

  return (
    <div className="space-y-6">
      <PageHeader title="Security Settings" headingTestId="security-page-title" />

      <Tabs defaultValue="password">
        <TabsList>
          <TabsTrigger value="password" data-testid="password-tab">
            Password
          </TabsTrigger>
          <TabsTrigger value="totp" data-testid="totp-tab">
            Two-Factor Auth
          </TabsTrigger>
        </TabsList>
        <TabsContent value="password" data-testid="password-section-title">
          <ChangePasswordForm />
        </TabsContent>
        <TabsContent value="totp" data-testid="totp-section-title">
          <TotpStatusCard
            onEnable={() =>
              navigate({ to: '/$realmId/user/security/totp-setup', params: { realmId } })
            }
            onDisable={() => setTotpDialog('disable')}
            onRegenerate={() => setTotpDialog('regenerate')}
          />
        </TabsContent>
      </Tabs>

      <Dialog open={totpDialog === 'disable'} onOpenChange={(open) => !open && setTotpDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Disable TOTP</DialogTitle>
          </DialogHeader>
          <TotpDisableForm
            onSuccess={handleDialogClose}
            onCancel={handleDialogClose}
            isForceTotpEnabled={false}
          />
        </DialogContent>
      </Dialog>

      <Dialog
        open={totpDialog === 'regenerate'}
        onOpenChange={(open) => !open && setTotpDialog(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Regenerate TOTP</DialogTitle>
          </DialogHeader>
          <TotpRegenerateForm onSuccess={handleDialogClose} onCancel={handleDialogClose} />
        </DialogContent>
      </Dialog>
    </div>
  )
}
