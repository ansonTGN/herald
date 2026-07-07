import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { ChangePasswordForm } from '@/components/profile/change-password-form'
import { TotpStatusCard } from '@/components/profile/totp/totp-status-card'
import { PasskeyList } from '@/components/profile/passkey/passkey-list'
import { PasskeyRegisterForm } from '@/components/profile/passkey/passkey-register-form'
import { DeleteAccountDialog } from '@/components/security/DeleteAccountDialog'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { TotpDisableForm } from '@/components/profile/totp/totp-disable-form'
import { TotpRegenerateForm } from '@/components/profile/totp/totp-regenerate-form'
import { PageHeader } from '@/components/shared'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card'
import { m } from '@/paraglide/messages'

export const Route = createFileRoute('/$realmId/user/security/')({
  component: ProfileSecurity,
})

type TotpDialogType = 'disable' | 'regenerate' | null

export function ProfileSecurity() {
  const navigate = useNavigate()
  const { realmId } = Route.useParams()
  const [totpDialog, setTotpDialog] = useState<TotpDialogType>(null)
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  // Toggles the inline passkey registration form within the passkey tab.
  const [passkeyRegistering, setPasskeyRegistering] = useState(false)

  const handleDialogClose = () => setTotpDialog(null)

  return (
    <div className="space-y-6">
      <PageHeader title={m['profile.security_page_title']()} headingTestId="security-page-title" />

      <Tabs defaultValue="password">
        <TabsList>
          <TabsTrigger value="password" data-testid="password-tab">
            {m['profile.password_tab']()}
          </TabsTrigger>
          <TabsTrigger value="totp" data-testid="totp-tab">
            {m['profile.totp_tab']()}
          </TabsTrigger>
          <TabsTrigger value="passkey" data-testid="passkey-tab">
            {m['profile.passkey_tab']()}
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
        <TabsContent value="passkey" data-testid="passkey-section-title">
          {passkeyRegistering ? (
            <PasskeyRegisterForm
              onSuccess={() => setPasskeyRegistering(false)}
              onCancel={() => setPasskeyRegistering(false)}
            />
          ) : (
            <div className="space-y-4">
              <div>
                <h2 className="text-2xl font-bold">{m['profile.passkey_title']()}</h2>
                <p className="text-muted-foreground">{m['profile.passkey_description']()}</p>
              </div>
              <PasskeyList onAdd={() => setPasskeyRegistering(true)} />
            </div>
          )}
        </TabsContent>
      </Tabs>

      <Dialog open={totpDialog === 'disable'} onOpenChange={(open) => !open && setTotpDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{m['profile.totp_disable_title']()}</DialogTitle>
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
            <DialogTitle>{m['profile.totp_regenerate_title']()}</DialogTitle>
          </DialogHeader>
          <TotpRegenerateForm onSuccess={handleDialogClose} onCancel={handleDialogClose} />
        </DialogContent>
      </Dialog>

      <Card data-testid="danger-operations-section">
        <CardHeader>
          <CardTitle>{m['security.delete_account.section_title']()}</CardTitle>
          <CardDescription>{m['security.delete_account.section_description']()}</CardDescription>
        </CardHeader>
        <CardContent>
          <Button
            variant="destructive"
            onClick={() => setDeleteDialogOpen(true)}
            data-testid="delete-account-open-button"
          >
            {m['security.delete_account.button']()}
          </Button>
        </CardContent>
      </Card>

      <DeleteAccountDialog
        realmId={realmId}
        open={deleteDialogOpen}
        onOpenChange={setDeleteDialogOpen}
      />
    </div>
  )
}
