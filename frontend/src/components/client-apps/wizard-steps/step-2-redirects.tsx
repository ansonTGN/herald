import { useState } from 'react'
import { ChevronDown, Settings } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Button } from '@/components/ui/button'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { RedirectUrisInput } from '../redirect-uris-input'
import { useWizardFormContext } from '../wizard-form-context'
import { transformToUriItems, transformFromUriItems } from './step-2-schema'

interface Step2RedirectsProps {
  mode: 'create' | 'edit'
}

/**
 * Step 2: Redirect URIs component
 *
 * Features:
 * - Valid Redirect URIs (required)
 * - Valid Post Logout URIs (optional)
 * - Web Origins (optional)
 * - Advanced CORS Settings (collapsible)
 */
export function Step2Redirects({ mode }: Step2RedirectsProps) {
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const { form, onNext } = useWizardFormContext()

  return (
    <div data-testid="redirect-uris-step" className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold mb-2">Redirect URIs</h2>
        <p className="text-sm text-muted-foreground">
          {mode === 'create'
            ? 'Configure the allowed OAuth redirect URIs for this application.'
            : 'Configure the allowed OAuth redirect URIs for this application.'}
        </p>
      </div>

      <div>
        {/* Valid Redirect URIs - Required */}
        <form.Field
          name="redirectUris"
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          children={(field: any) => (
            <div className="space-y-2">
              <RedirectUrisInput
                value={transformToUriItems(field.state.value ?? [])}
                onChange={(items) => {
                  const uris = transformFromUriItems(items)
                  field.handleChange(uris)
                }}
                label="Valid Redirect URIs"
                placeholder="https://example.com/callback"
                helpText="Enter the OAuth 2.0 redirect URIs. Each URI must be a valid URL starting with https:// (or http:// for development)."
                required
                dataTestId="redirect-uris"
                onSubmit={onNext}
              />
              {(field.state.meta.isTouched || form.state.isSubmitted) &&
                field.state.meta.errors.length > 0 && (
                  <p className="text-sm text-destructive flex items-center gap-1">
                    {getFieldErrorMessage(field.state.meta)}
                  </p>
                )}
            </div>
          )}
        />

        {/* Valid Post Logout Redirect URIs - Optional */}
        <form.Field
          name="postLogoutUris"
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          children={(field: any) => (
            <div className="space-y-2 mt-6">
              <RedirectUrisInput
                value={transformToUriItems(field.state.value ?? [])}
                onChange={(items) => {
                  const uris = transformFromUriItems(items)
                  field.handleChange(uris)
                }}
                label="Valid Post Logout URIs"
                placeholder="https://example.com/post-logout"
                helpText="Optional: URIs to redirect to after user logout. If not specified, the default redirect URI will be used."
                required={false}
                dataTestId="post-logout-uris"
              />
              {(field.state.meta.isTouched || form.state.isSubmitted) &&
                field.state.meta.errors.length > 0 && (
                  <p className="text-sm text-destructive flex items-center gap-1">
                    {getFieldErrorMessage(field.state.meta)}
                  </p>
                )}
            </div>
          )}
        />

        {/* Advanced CORS Settings - Collapsible */}
        <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
          <div className="mt-6">
            <CollapsibleTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                className="w-full justify-between px-3 py-2 h-auto hover:bg-accent"
                data-testid="advanced-cors-toggle"
                aria-expanded={advancedOpen}
                aria-controls="advanced-cors-content"
              >
                <div className="flex items-center gap-2">
                  <Settings className="w-4 h-4" aria-hidden="true" />
                  <span className="font-medium">Advanced CORS Settings</span>
                </div>
                <ChevronDown
                  className={cn(
                    'w-4 h-4 transition-transform',
                    advancedOpen && 'transform rotate-180'
                  )}
                  aria-hidden="true"
                />
              </Button>
            </CollapsibleTrigger>

            <CollapsibleContent className="pt-4">
              <div
                id="advanced-cors-content"
                className="rounded-lg border border-border bg-muted/50 p-4 space-y-4"
                data-testid="advanced-cors-content"
                role="region"
                aria-labelledby="advanced-cors-toggle"
              >
                <div>
                  <h3 className="text-sm font-medium mb-2">Web Origins</h3>
                  <p className="text-xs text-muted-foreground mb-3">
                    Allowed origins for Cross-Origin Resource Sharing (CORS). This controls which
                    domains can make API calls to this application.
                  </p>
                </div>

                <form.Field
                  name="webOrigins"
                  // eslint-disable-next-line @typescript-eslint/no-explicit-any
                  children={(field: any) => (
                    <div className="space-y-2">
                      <RedirectUrisInput
                        value={transformToUriItems(field.state.value ?? [])}
                        onChange={(items) => {
                          const uris = transformFromUriItems(items)
                          field.handleChange(uris)
                        }}
                        placeholder="https://example.com"
                        helpText="Enter allowed web origins (e.g., https://example.com). Use * to allow all origins (not recommended for production)."
                        required={false}
                        dataTestId="web-origins"
                      />
                      {(field.state.meta.isTouched || form.state.isSubmitted) &&
                        field.state.meta.errors.length > 0 && (
                          <p
                            className="text-sm text-destructive flex items-center gap-1"
                            role="alert"
                          >
                            {getFieldErrorMessage(field.state.meta)}
                          </p>
                        )}
                    </div>
                  )}
                />

                <div className="text-xs text-muted-foreground bg-background/50 rounded p-3">
                  <strong>Note:</strong> Web origins are used for CORS validation on API requests.
                  Make sure to include all domains that will interact with your application.
                </div>
              </div>
            </CollapsibleContent>
          </div>
        </Collapsible>
      </div>
    </div>
  )
}
