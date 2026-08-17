import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import { profileQueryOptions } from '@/data/query-options'
import { PageHeader } from '@/components/shared'
import { m } from '@/paraglide/messages'

export const Route = createFileRoute('/$realmId/user/profile')({
  component: ProfileIndex,
})

export function ProfileIndex() {
  const { data: profile, isLoading } = useQuery(profileQueryOptions)

  if (isLoading) {
    return <div>{m['profile.loading']()}</div>
  }

  if (!profile) {
    return <div>{m['profile.failed_to_load']()}</div>
  }

  return (
    <div className="space-y-6">
      <PageHeader title={m['profile.page_title']()} />

      {/* Profile Information Card */}
      <Card>
        <CardHeader>
          <CardTitle>{m['profile.info_card_title']()}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>{m['profile.email_label']()}</Label>
            <p className="text-sm text-muted-foreground" data-testid="email-display">
              {profile.email}
            </p>
          </div>
          <div className="space-y-2">
            <Label>{m['profile.nickname_label']()}</Label>
            <p className="text-sm text-muted-foreground" data-testid="nickname-display">
              {profile.nickname || m['profile.nickname_not_set']()}
            </p>
          </div>
          <div className="space-y-2">
            <Label>{m['profile.status_label']()}</Label>
            <p className="text-sm text-muted-foreground" data-testid="status-display">
              {profile.status === 1 ? m['profile.status_normal']() : m['profile.status_other']()}
            </p>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
