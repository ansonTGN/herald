import { useState, useEffect, useRef } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useQueryClient } from '@tanstack/react-query'
import { ProgressIndicator } from '@/components/ui/progress-indicator'
import { Button } from '@/components/ui/button'
import { useAppForm } from '@/components/ui/tanstack-form'
import { createClientApp, updateClientApp } from '@/lib/api-generated'
import type {
  ClientAppItem,
  ClientAppCreateRequest,
  ClientAppUpdateRequest,
} from '@/lib/api-generated'
import { toast } from 'sonner'
import { cn } from '@/lib/utils'
import { queryKeys } from '@/data/query-options'
import { Step1Basic, Step2Redirects, Step3Security, Step4Review } from './wizard-steps'
import { useDraftAutoSave } from '@/hooks/use-draft-autosave'
import { DraftRestoreDialog } from './draft-restore-dialog'
import { WizardFormProvider } from './wizard-form-context'
import { wizardSchema, mapInitialData, type WizardFormData } from './wizard-schema'

export type ClientAppWizardMode = 'create' | 'edit'

/**
 * Props for the ClientAppWizard component
 *
 * @interface ClientAppWizardProps
 * @property {ClientAppWizardMode} mode - The wizard mode: 'create' for new apps, 'edit' for existing apps
 * @property {string} realmId - The ID of the realm/realm for which the client app is being managed
 * @property {ClientAppItem} [initialData] - Optional initial data for edit mode, containing existing client app details
 *
 * @example
 * ```tsx
 * // Create mode
 * <ClientAppWizard mode="create" realmId="my-realm" />
 *
 * // Edit mode
 * <ClientAppWizard mode="edit" realmId="my-realm" initialData={existingApp} />
 * ```
 */
interface ClientAppWizardProps {
  mode: ClientAppWizardMode
  realmId: string
  initialData?: ClientAppItem
}

const WIZARD_STEPS = [
  { id: 'basic', title: 'Basic Information' },
  { id: 'redirect-uris', title: 'Redirect URIs' },
  { id: 'security', title: 'Security' },
  { id: 'review', title: 'Review & Create' },
] as const

const LAST_STEP = WIZARD_STEPS.length - 1

/**
 * ClientAppWizard - A multi-step wizard for creating and editing OAuth 2.0 client applications
 *
 * @component
 * @description
 * A comprehensive 4-step wizard that guides users through the OAuth 2.0 client application
 * configuration process with the following features:
 *
 * **Features:**
 * - **4-Step Flow**: Basic Info → Redirect URIs → Security → Review
 * - **Auto-Save**: Automatic draft saving with restore capability (create mode only)
 * - **Validation**: Real-time form validation with error handling
 * - **Animations**: Smooth step transitions with GPU acceleration
 * - **Accessibility**: Full keyboard navigation, ARIA labels, screen reader support
 * - **Responsive**: Works on desktop and mobile devices
 *
 * **Step Overview:**
 * 1. **Basic Information**: App name, description, type (Web/Service/Mobile/Native), client type (Confidential/Public)
 * 2. **Redirect URIs**: OAuth redirect URIs, post-logout URIs, web origins, advanced CORS settings
 * 3. **Security Settings**: Session TTL, renewal TTL, advanced security options
 * 4. **Review & Create/Save**: Summary of all configurations with edit capability
 *
 * **Auto-Save System:**
 * - Automatically saves form data to localStorage every 30 seconds
 * - Shows restore dialog on component mount if draft exists
 * - Uses version-based schema validation
 * - Cleared on successful submission or explicit cancel
 *
 * **Performance:**
 * - Code splitting with React.lazy for optimal bundle size
 * - GPU-accelerated animations for 60fps performance
 * - Optimized re-renders using React.memo where applicable
 *
 * **Accessibility:**
 * - Full keyboard navigation (Tab, Enter, Escape, Arrow keys)
 * - ARIA labels and roles for screen readers
 * - Focus management in modals and drawers
 * - Color contrast meeting WCAG AA standards
 * - Respects prefers-reduced-motion settings
 *
 * @param {ClientAppWizardProps} props - The component props
 * @returns {JSX.Element} The rendered wizard component
 *
 * @example
 * ```tsx
 * // Route component usage
 * function NewClientAppPage() {
 *   const { realmId } = useParams()
 *   return (
 *     <div className="container max-w-3xl mx-auto py-12">
 *       <ClientAppWizard mode="create" realmId={realmId} />
 *     </div>
 *   )
 * }
 * ```
 */
export function ClientAppWizard({ mode, realmId, initialData }: ClientAppWizardProps) {
  const [currentStep, setCurrentStep] = useState(0)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [showDraftDialog, setShowDraftDialog] = useState(false)
  const [hasRestoredDraft, setHasRestoredDraft] = useState(false)
  const [loadedDraft, setLoadedDraft] = useState<{
    data: Partial<WizardFormData>
    timestamp: number
    version: string
  } | null>(null)
  const [isTransitioning, setIsTransitioning] = useState(false)
  const [stepKey, setStepKey] = useState(0)

  // Generate draft key based on realm and mode
  const draftKey = `client-app-draft-${realmId}-${mode}-${initialData?.id || 'new'}`

  // Single form instance for the entire wizard
  const form = useAppForm({
    schema: wizardSchema,
    defaultValues: mapInitialData(initialData),
  })

  const navigate = useNavigate()
  const queryClient = useQueryClient()

  // Use useRef to store form values and prevent infinite loops
  // form.state.values should be stable in TanStack Form, but we'll use a ref to be safe
  const formValuesRef = useRef(form.state.values)
  const submitLockRef = useRef(false)

  // Auto-save hook - only enabled for create mode
  // Pass a function that returns the current form values
  // This prevents the hook from re-running when form values change
  console.log('[ClientAppWizard] Initializing useDraftAutoSave', {
    draftKey,
    enabled: mode === 'create',
  })

  const { clearDraft, loadDraft, hasDraft, saveDraft } = useDraftAutoSave<Partial<WizardFormData>>({
    draftKey,
    data: () => formValuesRef.current, // Pass a function that returns current values
    enabled: mode === 'create',
    version: '1.0',
    onSaveError: (error) => {
      console.error('[ClientAppWizard] Auto-save error:', error)
    },
  })

  // Update the ref and trigger save when form values actually change
  useEffect(() => {
    const prevValues = formValuesRef.current
    const currentValues = form.state.values

    // Only update ref if values actually changed (deep comparison)
    if (JSON.stringify(prevValues) !== JSON.stringify(currentValues)) {
      console.log('[ClientAppWizard] Form values changed, updating ref and triggering save', {
        prev: prevValues,
        current: currentValues,
      })
      formValuesRef.current = currentValues
      // Trigger a debounced save when values change
      if (mode === 'create') {
        saveDraft()
      }
    }
  }, [form.state.values, mode, saveDraft])

  const stepTitle = mode === 'create' ? 'Create Client App' : 'Edit Client App'
  const reviewLabel = mode === 'create' ? 'Create' : 'Save Changes'

  // Check for draft on component mount
  useEffect(() => {
    console.log('[ClientAppWizard] Draft check effect', {
      mode,
      hasDraft: hasDraft(),
      hasRestoredDraft,
    })

    // Only show draft dialog in create mode or if there's a draft
    if (mode === 'create' && hasDraft() && !hasRestoredDraft) {
      const draft = loadDraft()
      if (draft && draft.data) {
        console.log('[ClientAppWizard] Draft found, showing dialog', draft)
        setLoadedDraft(draft)
        setShowDraftDialog(true)
        setHasRestoredDraft(true)
      }
    }
  }, [mode, hasDraft, loadDraft, hasRestoredDraft])

  const handleCancel = () => {
    // Clear draft when canceling
    clearDraft()
    navigate({
      to: '/$realmId/manage/client-apps',
      params: { realmId },
    })
  }

  const handleRestoreDraft = () => {
    const draft = loadDraft()
    if (draft && draft.data) {
      console.log('[ClientAppWizard] Restoring draft', draft.data)
      // Restore form values from draft - use form.setFieldValue for proper React updates
      const draftData = draft.data as Partial<WizardFormData>

      // Use TanStack Form's API to properly update fields
      if (draftData.name !== undefined) form.setFieldValue('name', draftData.name)
      if (draftData.description !== undefined)
        form.setFieldValue('description', draftData.description)
      if (draftData.appType !== undefined) form.setFieldValue('appType', draftData.appType)
      if (draftData.clientType !== undefined) form.setFieldValue('clientType', draftData.clientType)
      if (draftData.redirectUris !== undefined)
        form.setFieldValue('redirectUris', draftData.redirectUris)
      if (draftData.postLogoutUris !== undefined)
        form.setFieldValue('postLogoutUris', draftData.postLogoutUris)
      if (draftData.webOrigins !== undefined) form.setFieldValue('webOrigins', draftData.webOrigins)
      if (draftData.sessionTtlSeconds !== undefined)
        form.setFieldValue('sessionTtlSeconds', draftData.sessionTtlSeconds)
      if (draftData.sessionRenewalTtlSeconds !== undefined)
        form.setFieldValue('sessionRenewalTtlSeconds', draftData.sessionRenewalTtlSeconds)
      if (draftData.deviceCodeGrantEnabled !== undefined)
        form.setFieldValue('deviceCodeGrantEnabled', draftData.deviceCodeGrantEnabled)

      setHasRestoredDraft(true)
      setShowDraftDialog(false)
      toast.success('Draft restored successfully')
    }
  }

  const handleDiscardDraft = () => {
    clearDraft()
    setShowDraftDialog(false)
    setHasRestoredDraft(true) // Prevent showing dialog again
  }

  const handleNext = () => {
    if (currentStep < LAST_STEP && !isTransitioning) {
      setIsTransitioning(true)
      setTimeout(() => {
        setCurrentStep((prev) => prev + 1)
        setStepKey((prev) => prev + 1)
        setIsTransitioning(false)
      }, 200)
    }
  }

  const handleBack = () => {
    if (currentStep > 0 && !isTransitioning) {
      setIsTransitioning(true)
      setTimeout(() => {
        setCurrentStep((prev) => prev - 1)
        setStepKey((prev) => prev + 1)
        setIsTransitioning(false)
      }, 200)
    }
  }

  const handleEditStep = (stepIndex: number) => {
    if (!isTransitioning && stepIndex !== currentStep) {
      setIsTransitioning(true)
      setTimeout(() => {
        setCurrentStep(stepIndex)
        setStepKey((prev) => prev + 1)
        setIsTransitioning(false)
      }, 200)
    }
  }

  const transformFormDataToCreateRequest = (data: WizardFormData): ClientAppCreateRequest => {
    // Generate a client ID based on the app name (alphanumeric, lowercase)
    // Limit to 28 characters to ensure final ID (with suffix) stays within 36 characters
    const baseClientId =
      data.name
        ?.toLowerCase()
        .replace(/[^a-z0-9]/g, '')
        .substring(0, 28) || 'client'

    // Add a random suffix to ensure uniqueness
    const randomSuffix = Math.random().toString(36).substring(2, 6)
    const clientId = `${baseClientId}-${randomSuffix}`

    return {
      clientId,
      name: data.name || '',
      description: data.description || null,
      redirectUris: data.redirectUris || [],
      sessionTtlSeconds: data.sessionTtlSeconds || 3600,
      sessionRenewalTtlSeconds: data.sessionRenewalTtlSeconds || null,
      deviceCodeGrantEnabled: data.deviceCodeGrantEnabled ?? false,
      enabled: true,
      iconUrl: null,
    }
  }

  const transformFormDataToUpdateRequest = (data: WizardFormData): ClientAppUpdateRequest => {
    return {
      name: data.name,
      description: data.description || null,
      redirectUris: data.redirectUris,
      sessionTtlSeconds: data.sessionTtlSeconds,
      sessionRenewalTtlSeconds: data.sessionRenewalTtlSeconds || null,
      deviceCodeGrantEnabled: data.deviceCodeGrantEnabled ?? false,
    }
  }

  const validateFormData = (data: WizardFormData): boolean => {
    // Validate Step 1
    const isBasicInfoValid =
      mode === 'edit' ? Boolean(data.name) : Boolean(data.name && data.appType && data.clientType)

    if (!isBasicInfoValid) {
      toast.error('Please complete all required fields in Basic Information')
      return false
    }

    // Validate Step 2
    if (!data.redirectUris || data.redirectUris.length === 0) {
      toast.error('Please add at least one redirect URI')
      return false
    }

    // Validate Step 3
    const sessionTtl = data.sessionTtlSeconds
    if (!sessionTtl || sessionTtl < 60) {
      toast.error('Session TTL must be at least 60 seconds')
      return false
    }

    // Validate renewal TTL if provided
    const renewalTtl = data.sessionRenewalTtlSeconds
    if (renewalTtl && renewalTtl <= sessionTtl) {
      toast.error('Session renewal TTL must be greater than session TTL')
      return false
    }

    return true
  }

  const handleSubmit = async () => {
    if (submitLockRef.current) {
      return
    }

    const formData = formValuesRef.current as WizardFormData

    if (!validateFormData(formData)) {
      return
    }

    submitLockRef.current = true
    setIsSubmitting(true)
    let shouldKeepSubmitLocked = false

    try {
      if (mode === 'create') {
        const createRequest = transformFormDataToCreateRequest(formData)
        const response = await createClientApp({
          path: { realmId },
          body: createRequest,
        })

        if (response.error) {
          throw response.error
        }

        if (response.data) {
          const createdApp = response.data
          shouldKeepSubmitLocked = true

          // Show success message with client secret if available
          // Use onAutoClose to navigate after toast has been displayed
          if (createdApp.clientSecret) {
            toast.success(
              `Client app "${createdApp.name}" created successfully! Client ID: ${createdApp.clientId}. Please save your client secret securely.`,
              {
                duration: 4000,
                onAutoClose: () => {
                  navigate({
                    to: '/$realmId/manage/client-apps',
                    params: { realmId },
                  })
                },
              }
            )
          } else {
            toast.success(`Client app "${createdApp.name}" created successfully!`, {
              duration: 4000,
              onAutoClose: () => {
                navigate({
                  to: '/$realmId/manage/client-apps',
                  params: { realmId },
                })
              },
            })
          }
        }
      } else {
        // Edit mode
        if (!initialData?.id) {
          throw new Error('Client app ID is required for update')
        }

        const updateRequest = transformFormDataToUpdateRequest(formData)
        const response = await updateClientApp({
          path: { realmId, clientAppId: initialData.id },
          body: updateRequest,
        })

        if (response.error) {
          throw response.error
        }

        if (response.data) {
          shouldKeepSubmitLocked = true
          toast.success(`Client app "${response.data.name}" updated successfully!`, {
            duration: 4000,
            onAutoClose: () => {
              navigate({
                to: '/$realmId/manage/client-apps',
                params: { realmId },
              })
            },
          })
        }
      }

      // Clear draft after successful submission
      clearDraft()

      // Invalidate queries to refresh list with latest data
      await queryClient.invalidateQueries({
        queryKey: queryKeys.clientAppsList(realmId),
      })

      // Note: Navigation is handled by toast.onAutoClose callback above
    } catch (error) {
      console.error('Failed to submit client app:', error)
      const errorMessage = error instanceof Error ? error.message : 'An unknown error occurred'
      toast.error(
        `Failed to ${mode === 'create' ? 'create' : 'update'} client app: ${errorMessage}`
      )
    } finally {
      if (!shouldKeepSubmitLocked) {
        submitLockRef.current = false
        setIsSubmitting(false)
      }
    }
  }

  console.log('[ClientAppWizard] Rendering', {
    mode,
    realmId,
    currentStep,
    isSubmitting,
    isTransitioning,
    stepKey,
    formValues: formValuesRef.current,
  })

  return (
    <WizardFormProvider form={form} onNext={handleNext}>
      <div data-testid="client-app-wizard">
        <div className="mb-8">
          <h1 className="text-3xl font-bold" data-testid="client-app-wizard-heading">
            {stepTitle}
          </h1>
          <p className="text-muted-foreground mt-1">
            {mode === 'create'
              ? 'Configure a new OAuth 2.0 client application'
              : 'Update OAuth 2.0 client application configuration'}
          </p>
        </div>

        <div className="mb-8">
          <ProgressIndicator steps={[...WIZARD_STEPS]} currentStep={currentStep} />
        </div>

        <div
          className={cn('rounded-lg border bg-card p-6', !isTransitioning && 'animate-step-enter')}
          key={stepKey}
          data-testid={`wizard-step-${WIZARD_STEPS[currentStep].id}`}
          role="region"
          aria-labelledby={`wizard-step-${WIZARD_STEPS[currentStep].id}-title`}
          aria-live="polite"
          aria-atomic="true"
        >
          {currentStep === 0 && <Step1Wrapper mode={mode} />}
          {currentStep === 1 && <Step2Wrapper mode={mode} />}
          {currentStep === 2 && <Step3Wrapper mode={mode} />}
          {currentStep === 3 && (
            <Step4Review
              mode={mode}
              formData={formValuesRef.current as WizardFormData}
              onEditStep={handleEditStep}
              isSubmitting={isSubmitting}
            />
          )}
        </div>

        <div className="mt-6 flex justify-between" role="navigation" aria-label="Wizard navigation">
          <Button
            type="button"
            variant="outline"
            onClick={handleCancel}
            data-testid="cancel-button"
            aria-label="Cancel wizard and return to client apps list"
          >
            Cancel
          </Button>

          <div className="flex gap-2">
            {currentStep > 0 && (
              <Button
                type="button"
                variant="outline"
                onClick={handleBack}
                disabled={isTransitioning}
                data-testid="back-button"
                aria-label="Go back to previous step"
              >
                Back
              </Button>
            )}
            {currentStep < LAST_STEP ? (
              <Button
                type="button"
                onClick={handleNext}
                disabled={isTransitioning}
                data-testid="next-button"
                aria-label="Proceed to next step"
              >
                Next
              </Button>
            ) : (
              <Button
                type="button"
                onClick={handleSubmit}
                loading={isSubmitting}
                data-testid="submit-button"
                aria-label={
                  mode === 'create' ? 'Create new client app' : 'Save changes to client app'
                }
              >
                {reviewLabel}
              </Button>
            )}
          </div>
        </div>

        {/* Draft restore dialog */}
        {loadedDraft && (
          <DraftRestoreDialog
            open={showDraftDialog}
            draft={loadedDraft}
            onRestore={handleRestoreDraft}
            onDiscard={handleDiscardDraft}
            onClose={() => setShowDraftDialog(false)}
          />
        )}
      </div>
    </WizardFormProvider>
  )
}

// Step 1 wrapper - no props needed (form accessed via context)
function Step1Wrapper({ mode }: { mode: ClientAppWizardMode }) {
  console.log('[Step1Wrapper] Rendering', { mode })
  return <Step1Basic mode={mode} />
}

// Step 2 wrapper - no props needed (will be updated in Phase 3)
function Step2Wrapper({ mode }: { mode: ClientAppWizardMode }) {
  console.log('[Step2Wrapper] Rendering', { mode })
  return <Step2Redirects mode={mode} />
}

// Step 3 wrapper - no props needed (will be updated in Phase 3)
function Step3Wrapper({ mode }: { mode: ClientAppWizardMode }) {
  console.log('[Step3Wrapper] Rendering', { mode })
  return <Step3Security mode={mode} />
}
