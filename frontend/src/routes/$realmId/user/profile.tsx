import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import { SubscriptionInfoCard } from '@/components/billing/subscription-info-card'
import {
  clientAppsQueryOptions,
  featureAvailabilityQueryOptions,
  profileQueryOptions,
} from '@/data/query-options'
import { PageHeader } from '@/components/shared'

export const Route = createFileRoute('/$realmId/user/profile')({
  component: ProfileIndex,
})

function ProfileIndex() {
  const { realmId } = Route.useParams()
  const { data: profile, isLoading } = useQuery(profileQueryOptions)
  const { data: features, isLoading: loadingFeatures } = useQuery(
    featureAvailabilityQueryOptions(realmId)
  )

  // Query client apps for subscription info
  const subscriptionVisible = features?.user.subscriptionVisible !== false
  const { data: clientAppsResponse, isLoading: loadingApps } = useQuery({
    ...clientAppsQueryOptions(realmId, { page: 0, pageSize: 20 }),
    enabled: subscriptionVisible,
  })

  const clientApps = clientAppsResponse?.items ?? []

  if (isLoading || loadingFeatures || (subscriptionVisible && loadingApps)) {
    return <div>Loading...</div>
  }

  if (!profile) {
    return <div>Failed to load profile</div>
  }

  return (
    <div className="space-y-6">
      <PageHeader title="Profile" />

      {/* Profile Information Card */}
      <Card>
        <CardHeader>
          <CardTitle>Profile Information</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Email</Label>
            <p className="text-sm text-muted-foreground" data-testid="email-display">
              {profile.email}
            </p>
          </div>
          <div className="space-y-2">
            <Label>Nickname</Label>
            <p className="text-sm text-muted-foreground" data-testid="nickname-display">
              {profile.nickname || 'Not set'}
            </p>
          </div>
          <div className="space-y-2">
            <Label>Status</Label>
            <p className="text-sm text-muted-foreground" data-testid="status-display">
              {profile.status === 1 ? 'Normal' : 'Other'}
            </p>
          </div>
        </CardContent>
      </Card>

      {subscriptionVisible && (
        <Card>
          <CardHeader>
            <CardTitle>Subscription Status</CardTitle>
          </CardHeader>
          <CardContent>
            {clientApps.length > 0 ? (
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {clientApps.map((app) => (
                  <SubscriptionInfoCard
                    key={app.id}
                    realmId={realmId}
                    clientAppId={app.id}
                    clientAppName={app.name}
                  />
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground" data-testid="no-subscriptions-message">
                You don't have any client apps with subscriptions.
              </p>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  )
}
