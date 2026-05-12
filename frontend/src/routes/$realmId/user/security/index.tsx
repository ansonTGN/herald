import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { ChangePasswordForm } from '@/components/profile/change-password-form'
import { TotpStatusCard } from '@/components/profile/totp/totp-status-card'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { TotpDisableForm } from '@/components/profile/totp/totp-disable-form'
import { TotpRegenerateForm } from '@/components/profile/totp/totp-regenerate-form'

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
    <div className="container mx-auto py-6 space-y-8">
      <h1 className="text-3xl font-bold" data-testid="security-page-title">
        Security Settings
      </h1>

      <section>
        <h2 className="text-xl font-semibold mb-4" data-testid="totp-section-title">
          Two-Factor Authentication
        </h2>
        <TotpStatusCard
          onEnable={() =>
            navigate({ to: '/$realmId/user/security/totp-setup', params: { realmId } })
          }
          onDisable={() => setTotpDialog('disable')}
          onRegenerate={() => setTotpDialog('regenerate')}
        />
      </section>

      <section>
        <h2 className="text-xl font-semibold mb-4" data-testid="password-section-title">
          Password
        </h2>
        <ChangePasswordForm />
      </section>

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
