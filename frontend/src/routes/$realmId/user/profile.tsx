import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { profileQueryOptions, queryKeys } from '@/data/query-options'
import { PageHeader } from '@/components/shared'
import { m } from '@/paraglide/messages'
import { updateProfile } from '@/lib/api-generated'
import { toast } from 'sonner'
import { getErrorMessage } from '@/lib/error-utils'
import { isValidCurrencyCode, normalizeCurrencyCode } from '@/lib/currency-utils'

export const Route = createFileRoute('/$realmId/user/profile')({
  component: ProfileIndex,
})

/**
 * Self-service preferred-currency override. The purchase page highlights this
 * currency when the entitlement supports it; clearing the override falls back
 * to the realm default currency. The wire body uses the tri-state contract:
 * `null` clears the override, a value sets it, and `nickname` is omitted so it
 * stays unchanged.
 */
function PreferredCurrencyCard({ currentOverride }: { currentOverride: string | null }) {
  const queryClient = useQueryClient()
  const [value, setValue] = useState(currentOverride ?? '')
  const [error, setError] = useState<string | null>(null)

  const mutation = useMutation({
    mutationFn: async (preferredCurrency: string | null) => {
      const response = await updateProfile({
        body: preferredCurrency === null ? { preferredCurrency: null } : { preferredCurrency },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (_data, preferredCurrency) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.profile() })
      toast.success(
        preferredCurrency === null
          ? m['profile.preferred_currency_cleared']()
          : m['profile.preferred_currency_saved']()
      )
    },
    onError: (error: unknown) => {
      toast.error(m['profile.preferred_currency_save_failed']({ message: getErrorMessage(error) }))
    },
  })

  const handleSave = () => {
    // ISO 4217 codes are uppercase; normalize before validating so a lowercase
    // entry is saved as the canonical code instead of being rejected.
    const normalized = normalizeCurrencyCode(value)
    if (normalized === '') return
    if (!isValidCurrencyCode(normalized)) {
      setError(m['profile.preferred_currency_invalid']())
      return
    }
    setError(null)
    mutation.mutate(normalized)
  }

  const hasOverride = currentOverride !== null

  return (
    <Card>
      <CardHeader>
        <CardTitle>{m['profile.preferred_currency_card_title']()}</CardTitle>
      </CardHeader>
      <CardContent className="max-w-lg space-y-4">
        <div className="space-y-2">
          <Label htmlFor="preferred-currency-input">
            {m['profile.preferred_currency_label']()}
          </Label>
          <p className="text-sm text-muted-foreground" data-testid="preferred-currency-current">
            {hasOverride ? currentOverride : m['profile.preferred_currency_not_set']()}
          </p>
          <p className="text-xs text-muted-foreground">{m['profile.preferred_currency_help']()}</p>
        </div>
        <div className="flex gap-2">
          <Input
            id="preferred-currency-input"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="USD"
            className="uppercase"
            data-testid="preferred-currency-input"
            aria-invalid={error !== null}
          />
          <Button
            onClick={handleSave}
            disabled={mutation.isPending || value.trim() === ''}
            data-testid="preferred-currency-save"
          >
            {mutation.isPending
              ? m['profile.preferred_currency_saving']()
              : m['profile.preferred_currency_save']()}
          </Button>
          <Button
            variant="outline"
            onClick={() => mutation.mutate(null)}
            disabled={!hasOverride || mutation.isPending}
            data-testid="preferred-currency-clear"
          >
            {m['profile.preferred_currency_clear']()}
          </Button>
        </div>
        {error && (
          <p className="text-sm text-destructive" data-testid="preferred-currency-error">
            {error}
          </p>
        )}
      </CardContent>
    </Card>
  )
}

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

      <PreferredCurrencyCard currentOverride={profile.preferredCurrency ?? null} />
    </div>
  )
}
