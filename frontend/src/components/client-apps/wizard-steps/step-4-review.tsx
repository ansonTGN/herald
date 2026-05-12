import { Check, AlertCircle, Edit, Globe, Lock, Clock, Shield } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { APP_TYPE_OPTIONS, CLIENT_TYPE_OPTIONS } from './step-1-schema'
import type { Step1FormData, Step2FormData, Step3FormData } from '.'
import type { ClientAppWizardMode } from '../client-app-wizard'

interface Step4ReviewProps {
  mode: ClientAppWizardMode
  formData: Partial<Step1FormData & Step2FormData & Step3FormData>
  onEditStep: (stepIndex: number) => void
  isSubmitting?: boolean
}

/**
 * Step 4: Review & Create component
 *
 * Features:
 * - Read-only display of all accumulated configuration
 * - Real-time validation status with green checkmarks
 * - Grouped display (Basic Info, Redirect URIs, Security Settings)
 * - Edit buttons to jump back to specific steps
 * - Professional summary layout with visual hierarchy
 */
export function Step4Review({
  mode,
  formData,
  onEditStep,
  isSubmitting = false,
}: Step4ReviewProps) {
  // Validation status for each section
  const isBasicInfoValid =
    mode === 'edit'
      ? Boolean(formData.name)
      : Boolean(formData.name && formData.appType && formData.clientType)
  const isRedirectUrisValid = Boolean(formData.redirectUris && formData.redirectUris.length > 0)
  const isSecurityValid = Boolean(formData.sessionTtlSeconds && formData.sessionTtlSeconds >= 60)

  const isFormValid = isBasicInfoValid && isRedirectUrisValid && isSecurityValid

  // Helper to get display label for enum values
  const getAppTypeLabel = (value?: string) => {
    return APP_TYPE_OPTIONS.find((opt) => opt.value === value)?.label || value
  }

  const getClientTypeLabel = (value?: string) => {
    return CLIENT_TYPE_OPTIONS.find((opt) => opt.value === value)?.label || value
  }

  const formatSessionDuration = (seconds?: number) => {
    if (!seconds) return '-'
    const hours = Math.floor(seconds / 3600)
    const minutes = Math.floor((seconds % 3600) / 60)
    if (hours > 0) {
      return `${hours}h ${minutes}m (${seconds}s)`
    }
    return `${minutes}m (${seconds}s)`
  }

  return (
    <div data-testid="review-step" className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold mb-2">
          Review & {mode === 'create' ? 'Create' : 'Save Changes'}
        </h2>
        <p className="text-sm text-muted-foreground">
          {mode === 'create'
            ? 'Review your configuration before creating the client application.'
            : 'Review your changes before updating the client application.'}
        </p>
      </div>

      {/* Validation Summary */}
      {!isFormValid && (
        <div className="flex items-start gap-2 p-3 bg-destructive/10 border border-destructive/20 rounded-lg">
          <AlertCircle className="w-4 h-4 text-destructive mt-0.5 flex-shrink-0" />
          <div className="text-sm text-destructive">
            <p className="font-medium">Please complete all required fields</p>
            <p className="text-xs mt-1">
              {!isBasicInfoValid && 'Basic Information, '}
              {!isRedirectUrisValid && 'Redirect URIs, '}
              {!isSecurityValid && 'Security Settings'}
            </p>
          </div>
        </div>
      )}

      {/* Basic Information Section */}
      <ReviewSection
        title="Basic Information"
        stepIndex={0}
        isValid={isBasicInfoValid}
        onEdit={onEditStep}
        data-testid="review-basic-info"
      >
        <ReviewRow label="App Name" value={formData.name} />
        <ReviewRow label="Description" value={formData.description || '-'} />
        <ReviewRow
          label="App Type"
          value={
            mode === 'edit'
              ? getAppTypeLabel(formData.appType) || 'Not editable for existing apps'
              : getAppTypeLabel(formData.appType)
          }
        />
        <ReviewRow
          label="Client Type"
          value={
            mode === 'edit'
              ? getClientTypeLabel(formData.clientType) || 'Not editable for existing apps'
              : getClientTypeLabel(formData.clientType)
          }
        />
      </ReviewSection>

      {/* Redirect URIs Section */}
      <ReviewSection
        title="Redirect URIs"
        stepIndex={1}
        isValid={isRedirectUrisValid}
        onEdit={onEditStep}
        icon={<Globe className="w-4 h-4" />}
        data-testid="review-redirect-uris"
      >
        <div className="space-y-3">
          <div>
            <p className="text-xs font-medium text-muted-foreground mb-1.5">Valid Redirect URIs</p>
            {formData.redirectUris && formData.redirectUris.length > 0 ? (
              <ul className="space-y-1">
                {formData.redirectUris.map((uri, index) => (
                  <li key={index} className="text-sm font-mono bg-muted/50 px-2 py-1 rounded">
                    {uri}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-sm text-muted-foreground">No redirect URIs configured</p>
            )}
          </div>

          {formData.postLogoutUris && formData.postLogoutUris.length > 0 && (
            <div>
              <p className="text-xs font-medium text-muted-foreground mb-1.5">Post Logout URIs</p>
              <ul className="space-y-1">
                {formData.postLogoutUris.map((uri, index) => (
                  <li key={index} className="text-sm font-mono bg-muted/50 px-2 py-1 rounded">
                    {uri}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {formData.webOrigins && formData.webOrigins.length > 0 && (
            <div>
              <p className="text-xs font-medium text-muted-foreground mb-1.5">Web Origins</p>
              <ul className="space-y-1">
                {formData.webOrigins.map((origin, index) => (
                  <li key={index} className="text-sm font-mono bg-muted/50 px-2 py-1 rounded">
                    {origin}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      </ReviewSection>

      {/* Security Settings Section */}
      <ReviewSection
        title="Security Settings"
        stepIndex={2}
        isValid={isSecurityValid}
        onEdit={onEditStep}
        icon={<Shield className="w-4 h-4" />}
        data-testid="review-security"
      >
        <ReviewRow
          label="Session TTL"
          value={formatSessionDuration(formData.sessionTtlSeconds)}
          icon={<Clock className="w-3 h-3" />}
        />
        <ReviewRow
          label="Session Renewal TTL"
          value={
            formData.sessionRenewalTtlSeconds
              ? formatSessionDuration(formData.sessionRenewalTtlSeconds)
              : 'Not configured'
          }
          icon={<Clock className="w-3 h-3" />}
        />
      </ReviewSection>

      {/* Submit Info */}
      <div className="pt-4 border-t">
        <div className="flex items-start gap-2 p-3 bg-muted/50 rounded-lg">
          <Lock className="w-4 h-4 text-muted-foreground mt-0.5 flex-shrink-0" />
          <div className="text-xs text-muted-foreground">
            <p className="font-medium text-foreground mb-1">Security Notice</p>
            <p>
              {mode === 'create'
                ? 'After creation, the client secret will be displayed once. Please store it securely as it will not be retrievable again.'
                : 'Changes will take effect immediately. Make sure to update your application configuration accordingly.'}
            </p>
          </div>
        </div>
      </div>

      {/* Form status for submission */}
      {isSubmitting && (
        <div className="text-sm text-muted-foreground text-center">Submitting...</div>
      )}
    </div>
  )
}

interface ReviewSectionProps {
  title: string
  stepIndex: number
  isValid: boolean
  onEdit: (stepIndex: number) => void
  icon?: React.ReactNode
  children: React.ReactNode
  'data-testid'?: string
}

function ReviewSection({
  title,
  stepIndex,
  isValid,
  onEdit,
  icon,
  children,
  'data-testid': dataTestId,
}: ReviewSectionProps) {
  return (
    <div className="rounded-lg border bg-card" data-testid={dataTestId}>
      <div className="flex items-center justify-between px-4 py-3 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          {icon}
          <h3 className="font-semibold text-sm">{title}</h3>
          {isValid ? (
            <Badge
              variant="outline"
              className="gap-1 text-xs bg-green-50 text-green-700 border-green-200"
            >
              <Check className="w-3 h-3" />
              Complete
            </Badge>
          ) : (
            <Badge
              variant="outline"
              className="gap-1 text-xs bg-destructive/10 text-destructive border-destructive/20"
            >
              <AlertCircle className="w-3 h-3" />
              Incomplete
            </Badge>
          )}
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => onEdit(stepIndex)}
          className="h-7 text-xs"
          data-testid={`edit-step-${stepIndex}`}
        >
          <Edit className="w-3 h-3 mr-1" />
          Edit
        </Button>
      </div>
      <div className="p-4 space-y-3">{children}</div>
    </div>
  )
}

interface ReviewRowProps {
  label: string
  value: string | number | undefined
  icon?: React.ReactNode
}

function ReviewRow({ label, value, icon }: ReviewRowProps) {
  return (
    <div className="flex items-start justify-between py-2">
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        {icon}
        <span>{label}</span>
      </div>
      <span className="text-sm font-medium text-right max-w-[60%] break-all">{value}</span>
    </div>
  )
}
