import { Info } from 'lucide-react'
import { cn } from '@/lib/utils'
import { AppForm } from '@/components/ui/tanstack-form'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { AdvancedSettingsCollapsible } from '../advanced-settings-collapsible'
import { SESSION_TTL_PRESETS, ADVANCED_SECURITY_OPTIONS } from './step-3-schema'
import type { ClientAppWizardMode } from '../client-app-wizard'
import { useWizardFormContext } from '../wizard-form-context'

interface Step3SecurityProps {
  mode: ClientAppWizardMode
}

/**
 * Step 3: Security Settings component
 *
 * Features:
 * - Session TTL configuration with presets
 * - Optional Session Renewal TTL
 * - Advanced Security Settings (collapsible, 8 options)
 * - Real-time validation
 * - Contextual help and explanations
 */
export function Step3Security({ mode }: Step3SecurityProps) {
  const { form, onNext } = useWizardFormContext()

  const sessionTtl = form.state.values.sessionTtlSeconds
  const renewalTtl = form.state.values.sessionRenewalTtlSeconds

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && onNext) {
      e.preventDefault()
      onNext()
    }
  }

  return (
    <div data-testid="security-step" className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold mb-2">Security Settings</h2>
        <p className="text-sm text-muted-foreground">
          {mode === 'create'
            ? 'Configure session and security policies for this client application.'
            : 'Manage security settings for this client application.'}
        </p>
      </div>

      <AppForm>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            e.stopPropagation()
            form.handleSubmit()
          }}
        >
          {/* Session TTL - Required */}
          <form.Field
            name="sessionTtlSeconds"
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            children={(field: any) => (
              <div className="space-y-3">
                <div>
                  <Label htmlFor="session-ttl" className="text-base font-medium">
                    Session Time-to-Live *
                  </Label>
                  <p className="text-xs text-muted-foreground mt-1">
                    How long a user session remains valid before requiring re-authentication
                  </p>
                </div>

                {/* Preset buttons */}
                <div className="grid grid-cols-4 gap-2">
                  {SESSION_TTL_PRESETS.map((preset) => (
                    <button
                      key={preset.value}
                      type="button"
                      onClick={() => {
                        field.handleChange(preset.seconds)
                      }}
                      className={cn(
                        'px-3 py-2 text-sm rounded-lg border transition-all',
                        'hover:bg-accent focus:outline-none focus:ring-2 focus:ring-ring',
                        field.state.value === preset.seconds
                          ? 'border-primary bg-primary/10 text-primary font-medium'
                          : 'border-border'
                      )}
                      data-testid={`session-ttl-preset-${preset.seconds}`}
                    >
                      {preset.label}
                    </button>
                  ))}
                </div>

                {/* Custom value input */}
                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <div className="space-y-2">
                      <input
                        type="number"
                        id="session-ttl-custom"
                        data-testid="session-ttl-custom-field"
                        placeholder="Custom value in seconds"
                        min={60}
                        max={86400}
                        value={field.state.value}
                        onChange={(e) => {
                          field.handleChange(Number(e.target.value))
                        }}
                        onKeyDown={handleKeyDown}
                        className={cn(
                          'w-full px-4 py-2.5 rounded-lg border border-input bg-background transition-all',
                          'focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent',
                          'placeholder:text-muted-foreground',
                          field.state.meta.errors.length > 0 &&
                            'border-destructive focus:ring-destructive'
                        )}
                        aria-invalid={field.state.meta.errors.length > 0}
                      />
                      <p className="text-xs text-muted-foreground">
                        Enter custom session duration in seconds (60-86400)
                      </p>
                    </div>
                  </div>
                  <div className="text-sm text-muted-foreground flex-shrink-0 pt-6">
                    {field.state.value && (
                      <span className="font-medium">
                        {Math.floor(field.state.value / 60)} minutes ({field.state.value}s)
                      </span>
                    )}
                  </div>
                </div>

                {(field.state.meta.isTouched || form.state.isSubmitted) &&
                  field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-destructive flex items-center gap-1" role="alert">
                      {getFieldErrorMessage(field.state.meta)}
                    </p>
                  )}

                {/* Info box */}
                <div
                  className="flex items-start gap-2 p-3 bg-muted/50 rounded-lg border border-border"
                  role="note"
                  aria-label="Information about Session TTL"
                >
                  <Info
                    className="w-4 h-4 text-muted-foreground mt-0.5 flex-shrink-0"
                    aria-hidden="true"
                  />
                  <div className="text-xs text-muted-foreground">
                    <p className="font-medium mb-1">About Session TTL</p>
                    <p>
                      Shorter sessions provide better security but require users to authenticate
                      more frequently. Longer sessions improve user experience but increase security
                      risk if a session is compromised.
                    </p>
                  </div>
                </div>
              </div>
            )}
          />

          {/* Session Renewal TTL - Optional */}
          <form.Field
            name="sessionRenewalTtlSeconds"
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            children={(field: any) => (
              <div className="space-y-3 mt-6">
                <div>
                  <Label htmlFor="session-renewal-ttl" className="text-base font-medium">
                    Session Renewal Time-to-Live
                  </Label>
                  <p className="text-xs text-muted-foreground mt-1">
                    Optional: Sliding session window for silent renewal (must be greater than
                    Session TTL)
                  </p>
                </div>

                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <div className="space-y-2">
                      <input
                        type="number"
                        id="session-renewal-ttl"
                        data-testid="session-renewal-ttl-field"
                        placeholder="e.g., 7200 (2 hours)"
                        min={0}
                        max={604800}
                        value={field.state.value ?? ''}
                        onChange={(e) => {
                          field.handleChange(e.target.value ? Number(e.target.value) : undefined)
                        }}
                        onKeyDown={handleKeyDown}
                        className={cn(
                          'w-full px-4 py-2.5 rounded-lg border border-input bg-background transition-all',
                          'focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent',
                          'placeholder:text-muted-foreground',
                          field.state.meta.errors.length > 0 &&
                            'border-destructive focus:ring-destructive'
                        )}
                        aria-invalid={field.state.meta.errors.length > 0}
                      />
                      <p className="text-xs text-muted-foreground">
                        Optional: Enter renewal window in seconds (must exceed Session TTL)
                      </p>
                    </div>
                  </div>
                  <div className="text-sm text-muted-foreground flex-shrink-0 pt-6">
                    {field.state.value && (
                      <span className="font-medium">
                        {Math.floor(field.state.value / 60)} minutes ({field.state.value}s)
                      </span>
                    )}
                  </div>
                </div>

                {/* Validation warning */}
                {sessionTtl && renewalTtl && renewalTtl <= sessionTtl && (
                  <p className="text-sm text-destructive flex items-center gap-1" role="alert">
                    Renewal TTL must be greater than Session TTL ({sessionTtl}s)
                  </p>
                )}

                {(field.state.meta.isTouched || form.state.isSubmitted) &&
                  field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-destructive flex items-center gap-1" role="alert">
                      {getFieldErrorMessage(field.state.meta)}
                    </p>
                  )}

                {/* Info box */}
                <div
                  className="flex items-start gap-2 p-3 bg-muted/50 rounded-lg border border-border"
                  role="note"
                  aria-label="Information about Session Renewal TTL"
                >
                  <Info
                    className="w-4 h-4 text-muted-foreground mt-0.5 flex-shrink-0"
                    aria-hidden="true"
                  />
                  <div className="text-xs text-muted-foreground">
                    <p className="font-medium mb-1">About Session Renewal TTL</p>
                    <p>
                      When set, users can refresh their sessions within this window without full
                      re-authentication. Creates a "sliding session" experience. If not set, users
                      must re-authenticate when the session expires.
                    </p>
                  </div>
                </div>
              </div>
            )}
          />

          {/* Device Code Grant Toggle */}
          <form.Field
            name="deviceCodeGrantEnabled"
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            children={(field: any) => (
              <div className="mt-6 p-4 bg-background rounded-lg border border-border">
                <div className="flex items-center justify-between">
                  <div className="flex-1">
                    <Label htmlFor="device-code-grant" className="font-medium text-sm">
                      Device Code Grant
                    </Label>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      Allow this client app to use the Device Authorization Grant flow (RFC 8628)
                    </p>
                  </div>
                  <Switch
                    id="device-code-grant"
                    data-testid="device-code-grant-switch"
                    checked={field.state.value ?? false}
                    onCheckedChange={field.handleChange}
                  />
                </div>
              </div>
            )}
          />

          {/* Advanced Security Settings - Collapsible */}
          <AdvancedSettingsCollapsible dataTestId="advanced-security-settings">
            <div>
              <h3 className="text-sm font-medium mb-2">Advanced Security Options</h3>
              <p className="text-xs text-muted-foreground mb-4">
                Fine-grained security controls for advanced OAuth 2.0 configurations. These options
                will be available when the backend API supports them.
              </p>
            </div>

            {/* Display placeholder for future options */}
            <div className="space-y-3">
              {ADVANCED_SECURITY_OPTIONS.map((option) => (
                <div
                  key={option.id}
                  className="flex items-center justify-between p-3 bg-background rounded-lg border border-border opacity-50"
                  data-testid={`advanced-option-${option.id}`}
                >
                  <div className="flex-1">
                    <Label htmlFor={option.id} className="font-medium text-sm">
                      {option.label}
                    </Label>
                    <p className="text-xs text-muted-foreground mt-0.5">{option.description}</p>
                  </div>
                  <Switch disabled id={option.id} checked={option.default} />
                </div>
              ))}
            </div>

            <div className="text-xs text-muted-foreground bg-background/50 rounded p-3 border border-border">
              <strong>Note:</strong> These advanced security options will be available in a future
              update. The backend API is being extended to support these configurations.
            </div>
          </AdvancedSettingsCollapsible>
        </form>
      </AppForm>
    </div>
  )
}
