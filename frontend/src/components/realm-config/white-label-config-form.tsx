import { useEffect, useRef, useState } from 'react'
import { useStore } from '@tanstack/react-form'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  whiteLabelConfigSchema,
  whiteLabelBackgroundSchema,
  type WhiteLabelConfigForm as WhiteLabelConfigFormValues,
  type WhiteLabelBackgroundForm,
} from '@/lib/schemas/realm-config'
import { getContrastRatio, WCAG_AA_MIN_CONTRAST } from '@/lib/white-label-contrast'
import type { PublicWhiteLabelConfig } from '@/lib/api-generated/types.gen'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { TextField } from '@/components/shared/form-fields/text-field'
import { m } from '@/paraglide/messages'

/**
 * Background type option values. The empty string is the "no background"
 * sentinel: when selected, `background` collapses to `null` so the preview
 * falls back to the default Herald gradient.
 */
const BACKGROUND_NONE = 'none'
const BACKGROUND_IMAGE = 'image'
const BACKGROUND_GRADIENT = 'gradient'
type BackgroundTypeOption =
  | typeof BACKGROUND_NONE
  | typeof BACKGROUND_IMAGE
  | typeof BACKGROUND_GRADIENT

export interface WhiteLabelConfigFormProps {
  /** Initial form values. Use `emptyWhiteLabelConfig()` when nothing is configured yet. */
  initialConfig: WhiteLabelConfigFormValues
  /** Whether an unpublished draft exists on the backend (shows the draft notice). */
  hasDraft?: boolean
  /** Whether a previous version can be restored. Gates the restore button. */
  hasPrevious?: boolean
  /** Disables all inputs + action buttons (e.g. missing `settings.manage`). */
  disabled?: boolean

  /**
   * Persist the current form values as a draft. Receives the already-normalized
   * PUT `/draft` / POST `/publish` request body so FE-D05 can pass it straight
   * to the generated client. Returns a promise so the form can track in-flight
   * state; rejections surface as the save error.
   */
  onSaveDraft: (config: WhiteLabelConfigFormValues) => void | Promise<void>
  /** Publish the current form values (writes `settings`). */
  onPublish: (config: WhiteLabelConfigFormValues) => void | Promise<void>
  /** Discard the saved draft and reset the editor to the published config. */
  onDiscardDraft: () => void | Promise<void>
  /** Restore the previous published config. Requires `hasPrevious`. */
  onRestore: () => void | Promise<void>

  /** In-flight flags driven by the parent's mutations. */
  isSavingDraft?: boolean
  isPublishing?: boolean
  isDiscarding?: boolean
  isRestoring?: boolean
}

/**
 * Maps the nullable admin form values to the public white-label shape consumed
 * by `AuthPageWrapper`. `PublicWhiteLabelConfig` treats every field as
 * optional+nullable, so we forward values as-is; null/empty values make the
 * wrapper fall back to Herald defaults.
 */
function toPublicWhiteLabel(formValues: WhiteLabelConfigFormValues): PublicWhiteLabelConfig {
  return {
    brandName: formValues.brandName,
    logoUrl: formValues.logoUrl,
    faviconUrl: formValues.faviconUrl,
    accentColor: formValues.accentColor,
    background: formValues.background,
    footerText: formValues.footerText,
    loginTitle: formValues.loginTitle,
    loginSubtitle: formValues.loginSubtitle,
    registerTitle: formValues.registerTitle,
    registerSubtitle: formValues.registerSubtitle,
  }
}

/**
 * Reads the background "type" select value from the stored background object.
 * A null background maps to the `none` option. A non-null background keeps its
 * type even when the value is empty: while editing, the user selects a type
 * first (which sets `{ type, value: '' }`) and then fills the value, so the
 * value Textarea must stay mounted across that empty-value state. Empty values
 * are normalized to `null` at save time in `realm-config-utils.normalizeBackground`.
 */
function backgroundTypeOption(background: WhiteLabelBackgroundForm | null): BackgroundTypeOption {
  if (!background) return BACKGROUND_NONE
  return background.type === BACKGROUND_IMAGE ? BACKGROUND_IMAGE : BACKGROUND_GRADIENT
}

export function WhiteLabelConfigForm({
  initialConfig,
  hasDraft = false,
  hasPrevious = false,
  disabled = false,
  onSaveDraft,
  onPublish,
  onDiscardDraft,
  onRestore,
  isSavingDraft = false,
  isPublishing = false,
  isDiscarding = false,
  isRestoring = false,
}: WhiteLabelConfigFormProps) {
  const [restoreDialogOpen, setRestoreDialogOpen] = useState(false)

  const form = useAppForm({
    schema: whiteLabelConfigSchema,
    defaultValues: initialConfig,
    onSubmit: async ({ value }) => {
      // The primary submit target is "save draft". Publish/discard/restore are
      // explicit buttons with their own handlers; keeping a single default
      // submit matches the existing config-form pattern (Enter submits draft).
      await onSaveDraft(value)
    },
  })

  // Keep the form in sync with the persisted source (design §5.5). `initialConfig`
  // is derived upstream from `draft ?? published`, so it only changes value when
  // the backend state changes externally (e.g. after discard the draft is gone
  // and the source flips back to `published`, or after restore/publish). We
  // compare by value (not reference) because the parent rebuilds the object on
  // every render, and only reseed the form when the source value actually
  // differs from the last one we applied — so in-flight edits are never wiped.
  const sourceKey = JSON.stringify(initialConfig)
  const lastSyncedSourceRef = useRef(sourceKey)
  useEffect(() => {
    if (sourceKey !== lastSyncedSourceRef.current) {
      lastSyncedSourceRef.current = sourceKey
      form.reset(initialConfig)
    }
  }, [sourceKey, initialConfig, form])

  // Subscribe to the live form values so the preview and contrast warning
  // update as the user types without re-rendering on unrelated state changes.
  const values = useStore(form.store, (state) => state.values)
  const isDirty = useStore(form.store, (state) => state.isDirty)

  const accentColor = values.accentColor ?? ''
  const accentRatio = accentColor ? getContrastRatio(accentColor) : NaN
  const showAccentWarning =
    accentColor.trim() !== '' && !Number.isNaN(accentRatio) && accentRatio < WCAG_AA_MIN_CONTRAST

  const showDraftNotice = hasDraft || isDirty
  const publicWhiteLabel = toPublicWhiteLabel(values)

  // --- Background field helpers -------------------------------------------------
  // The background is a {type,value} object on the schema but edited through a
  // type select + value input. Changing the type preserves the value when
  // switching between image/gradient and clears it when going to "none".
  const handleBackgroundTypeChange = (option: BackgroundTypeOption) => {
    if (option === BACKGROUND_NONE) {
      form.setFieldValue('background', null)
      return
    }
    const current = values.background
    const preservedValue = current?.value ?? ''
    const next: WhiteLabelBackgroundForm = {
      type: option,
      value: preservedValue,
    }
    // Validate through the schema-derived background object so an invalid type
    // can never be written.
    const parsed = whiteLabelBackgroundSchema.safeParse(next)
    if (parsed.success) {
      form.setFieldValue('background', parsed.data)
    }
  }

  const handleBackgroundValueChange = (value: string) => {
    // Keep the current type; an empty value keeps the object but will normalize
    // to null on save (see normalizeBackground in realm-config-utils).
    const type = backgroundTypeOption(values.background)
    if (type === BACKGROUND_NONE) return
    const next: WhiteLabelBackgroundForm = { type, value }
    form.setFieldValue('background', next)
  }

  // --- Action handlers ---------------------------------------------------------
  // Each action reads the *current* form values (not stale closure values) via
  // form.store so the parent always receives the latest edit. Validation is
  // delegated to the schema; on invalid values the action is skipped.
  const handlePublish = () => {
    if (disabled) return
    void onPublish(values)
  }

  const handleDiscardDraft = () => {
    if (disabled) return
    void onDiscardDraft()
  }

  const handleRestoreConfirm = () => {
    setRestoreDialogOpen(false)
    void onRestore()
  }

  return (
    <div className="grid gap-6 lg:grid-cols-2">
      {/* ============ Editor column ============ */}
      <Card>
        <CardHeader>
          <CardTitle>{m['settings.white_label.title']()}</CardTitle>
          <CardDescription>{m['settings.white_label.description']()}</CardDescription>
        </CardHeader>
        <CardContent>
          <AppForm>
            <form
              onSubmit={(e) => {
                e.preventDefault()
                form.handleSubmit()
              }}
              className="space-y-4"
            >
              <TextField
                form={form}
                name="brandName"
                label={m['settings.white_label.brand_name_label']()}
                inputId="white-label-brand-name"
                dataTestId="white-label-brand-name"
                placeholder="Acme"
                disabled={disabled}
                helpText={m['settings.white_label.brand_name_help']()}
              />

              <TextField
                form={form}
                name="logoUrl"
                label={m['settings.white_label.logo_url_label']()}
                inputId="white-label-logo-url"
                dataTestId="white-label-logo-url"
                type="url"
                placeholder="https://cdn.example.com/logo.svg"
                disabled={disabled}
                helpText={m['settings.white_label.logo_url_help']()}
              />

              <TextField
                form={form}
                name="faviconUrl"
                label={m['settings.white_label.favicon_url_label']()}
                inputId="white-label-favicon-url"
                dataTestId="white-label-favicon-url"
                type="url"
                placeholder="https://cdn.example.com/favicon.ico"
                disabled={disabled}
                helpText={m['settings.white_label.favicon_url_help']()}
              />

              {/* Accent color: color picker + hex input */}
              <form.Field
                name="accentColor"
                children={(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="white-label-accent-color">
                      {m['settings.white_label.accent_color_label']()}
                    </Label>
                    <div className="flex items-center gap-2">
                      <Input
                        id="white-label-accent-color-picker"
                        type="color"
                        value={normalizeColorInputValue(field.state.value)}
                        onChange={(e) => field.handleChange(e.target.value || null)}
                        disabled={disabled}
                        className="h-10 w-14 cursor-pointer p-1"
                        data-testid="white-label-accent-color-picker"
                      />
                      <Input
                        id="white-label-accent-color"
                        value={field.state.value ?? ''}
                        onChange={(e) => field.handleChange(e.target.value || null)}
                        placeholder="#2563eb"
                        disabled={disabled}
                        data-testid="white-label-accent-color"
                      />
                    </div>
                    <p className="text-xs text-muted-foreground">
                      {m['settings.white_label.accent_color_help']()}
                    </p>
                    {showAccentWarning && (
                      <p className="text-sm text-warning" data-testid="white-label-accent-warning">
                        {m['settings.white_label.accent_warning']({
                          ratio: accentRatio.toFixed(2),
                          min: WCAG_AA_MIN_CONTRAST,
                        })}
                      </p>
                    )}
                  </div>
                )}
              />

              {/* Background type */}
              <div className="space-y-2">
                <Label htmlFor="white-label-background-type">
                  {m['settings.white_label.background_type_label']()}
                </Label>
                <Select
                  value={backgroundTypeOption(values.background)}
                  onValueChange={(value) =>
                    handleBackgroundTypeChange(value as BackgroundTypeOption)
                  }
                  disabled={disabled}
                >
                  <SelectTrigger
                    id="white-label-background-type"
                    data-testid="white-label-background-type"
                  >
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value={BACKGROUND_NONE}>
                      {m['settings.white_label.background_type_none']()}
                    </SelectItem>
                    <SelectItem value={BACKGROUND_IMAGE}>
                      {m['settings.white_label.background_type_image']()}
                    </SelectItem>
                    <SelectItem value={BACKGROUND_GRADIENT}>
                      {m['settings.white_label.background_type_gradient']()}
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {/* Background value (only when a type is selected) */}
              {backgroundTypeOption(values.background) !== BACKGROUND_NONE && (
                <form.Field
                  name="background"
                  children={(field) => (
                    <div className="space-y-2">
                      <Label htmlFor="white-label-background-value">
                        {field.state.value?.type === BACKGROUND_GRADIENT
                          ? m['settings.white_label.background_value_gradient_label']()
                          : m['settings.white_label.background_value_image_label']()}
                      </Label>
                      <Textarea
                        id="white-label-background-value"
                        value={field.state.value?.value ?? ''}
                        onChange={(e) => handleBackgroundValueChange(e.target.value)}
                        placeholder={
                          field.state.value?.type === BACKGROUND_GRADIENT
                            ? 'linear-gradient(135deg, #1e3a8a, #2563eb)'
                            : 'https://cdn.example.com/bg.jpg'
                        }
                        disabled={disabled}
                        data-testid="white-label-background-value"
                        className="min-h-[64px]"
                      />
                      <p className="text-xs text-muted-foreground">
                        {field.state.value?.type === BACKGROUND_GRADIENT
                          ? m['settings.white_label.background_value_gradient_help']()
                          : m['settings.white_label.background_value_image_help']()}
                      </p>
                    </div>
                  )}
                />
              )}

              {/* Footer text */}
              <form.Field
                name="footerText"
                children={(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="white-label-footer-text">
                      {m['settings.white_label.footer_text_label']()}
                    </Label>
                    <Input
                      id="white-label-footer-text"
                      value={field.state.value ?? ''}
                      onChange={(e) => field.handleChange(e.target.value || null)}
                      placeholder="© Example Inc."
                      disabled={disabled}
                      data-testid="white-label-footer-text"
                    />
                  </div>
                )}
              />

              {/* Login copy */}
              <div className="space-y-4 rounded-md border p-4">
                <p className="text-sm font-medium">
                  {m['settings.white_label.login_section_title']()}
                </p>
                <TextField
                  form={form}
                  name="loginTitle"
                  label={m['settings.white_label.login_title_label']()}
                  inputId="white-label-login-title"
                  dataTestId="white-label-login-title"
                  placeholder="Sign in to Example"
                  disabled={disabled}
                />
                <TextField
                  form={form}
                  name="loginSubtitle"
                  label={m['settings.white_label.login_subtitle_label']()}
                  inputId="white-label-login-subtitle"
                  dataTestId="white-label-login-subtitle"
                  placeholder="Use your Example account"
                  disabled={disabled}
                />
              </div>

              {/* Register copy */}
              <div className="space-y-4 rounded-md border p-4">
                <p className="text-sm font-medium">
                  {m['settings.white_label.register_section_title']()}
                </p>
                <TextField
                  form={form}
                  name="registerTitle"
                  label={m['settings.white_label.register_title_label']()}
                  inputId="white-label-register-title"
                  dataTestId="white-label-register-title"
                  placeholder="Create your Example account"
                  disabled={disabled}
                />
                <TextField
                  form={form}
                  name="registerSubtitle"
                  label={m['settings.white_label.register_subtitle_label']()}
                  inputId="white-label-register-subtitle"
                  dataTestId="white-label-register-subtitle"
                  placeholder="Start with Example"
                  disabled={disabled}
                />
              </div>

              {showDraftNotice && (
                <p className="text-sm text-warning" data-testid="white-label-draft-notice">
                  {m['settings.white_label.draft_notice']()}
                </p>
              )}

              {/* Action buttons */}
              <div className="flex flex-wrap gap-2 pt-2">
                <Button
                  type="submit"
                  disabled={disabled || isSavingDraft}
                  data-testid="white-label-save-draft"
                >
                  {isSavingDraft
                    ? m['settings.white_label.saving']()
                    : m['settings.white_label.save_draft']()}
                </Button>
                <Button
                  type="button"
                  disabled={disabled || isPublishing}
                  data-testid="white-label-publish"
                  onClick={handlePublish}
                >
                  {isPublishing
                    ? m['settings.white_label.publishing']()
                    : m['settings.white_label.publish']()}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  disabled={disabled || isDiscarding || !hasDraft}
                  data-testid="white-label-discard-draft"
                  onClick={handleDiscardDraft}
                >
                  {isDiscarding
                    ? m['settings.white_label.discarding']()
                    : m['settings.white_label.discard_draft']()}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  disabled={disabled || isRestoring || !hasPrevious}
                  data-testid="white-label-restore"
                  onClick={() => setRestoreDialogOpen(true)}
                >
                  {isRestoring
                    ? m['settings.white_label.restoring']()
                    : m['settings.white_label.restore']()}
                </Button>
              </div>
            </form>
          </AppForm>
        </CardContent>
      </Card>

      {/* ============ Preview column ============ */}
      <Card>
        <CardHeader>
          <CardTitle>{m['settings.white_label.preview_title']()}</CardTitle>
          <CardDescription>{m['settings.white_label.preview_description']()}</CardDescription>
        </CardHeader>
        <CardContent>
          <Tabs defaultValue="login">
            <TabsList>
              <TabsTrigger value="login" data-testid="white-label-preview-login">
                {m['settings.white_label.preview_login_tab']()}
              </TabsTrigger>
              <TabsTrigger value="register" data-testid="white-label-preview-register">
                {m['settings.white_label.preview_register_tab']()}
              </TabsTrigger>
            </TabsList>
            <TabsContent value="login">
              <div
                className="overflow-hidden rounded-md border"
                data-testid="white-label-preview-login-panel"
              >
                <AuthPageWrapper whiteLabel={publicWhiteLabel}>
                  <PreviewCard title={values.loginTitle} subtitle={values.loginSubtitle} />
                </AuthPageWrapper>
              </div>
            </TabsContent>
            <TabsContent value="register">
              <div
                className="overflow-hidden rounded-md border"
                data-testid="white-label-preview-register-panel"
              >
                <AuthPageWrapper whiteLabel={publicWhiteLabel}>
                  <PreviewCard title={values.registerTitle} subtitle={values.registerSubtitle} />
                </AuthPageWrapper>
              </div>
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>

      {/* ============ Restore confirmation dialog ============ */}
      <AlertDialog open={restoreDialogOpen} onOpenChange={setRestoreDialogOpen}>
        <AlertDialogContent data-testid="white-label-restore-dialog">
          <AlertDialogHeader>
            <AlertDialogTitle>{m['settings.white_label.restore_dialog_title']()}</AlertDialogTitle>
            <AlertDialogDescription>
              {m['settings.white_label.restore_dialog_description']()}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isRestoring}>{m['common.cancel']()}</AlertDialogCancel>
            <AlertDialogAction
              disabled={isRestoring}
              onClick={(e) => {
                e.preventDefault()
                handleRestoreConfirm()
              }}
              data-testid="white-label-restore-confirm"
            >
              {isRestoring
                ? m['settings.white_label.restoring']()
                : m['settings.white_label.restore']()}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}

/**
 * Mini card rendered inside the preview's `AuthPageWrapper` to surface the
 * login/register title + subtitle. Kept deliberately lightweight: the real
 * auth pages own their own copy/layout; the preview only needs to show where
 * the brand assets land.
 */
function PreviewCard({ title, subtitle }: { title: string | null; subtitle: string | null }) {
  return (
    <div className="w-full max-w-sm rounded-lg border bg-card p-6 text-center text-card-foreground shadow-sm">
      <h1 className="text-lg font-semibold">
        {title || m['settings.white_label.preview_default_title']()}
      </h1>
      {subtitle ? <p className="mt-1 text-sm text-muted-foreground">{subtitle}</p> : null}
      <div className="mt-4 h-9 rounded-md bg-muted" aria-hidden="true" />
      <div className="mt-2 h-9 rounded-md bg-muted" aria-hidden="true" />
    </div>
  )
}

/**
 * The native color input requires a 7-char `#rrggbb` value; a 4-char `#rgb` or
 * empty/null value would reset the picker. Expand shorthand and fall back to
 * `#000000` when the value isn't a parseable hex so the picker never breaks.
 */
function normalizeColorInputValue(value: string | null | undefined): string {
  if (!value) return '#000000'
  const trimmed = value.trim()
  if (/^#[0-9a-fA-F]{6}$/.test(trimmed)) return trimmed.toLowerCase()
  if (/^#[0-9a-fA-F]{3}$/.test(trimmed)) {
    const [, r, g, b] = trimmed
    return `#${r}${r}${g}${g}${b}${b}`.toLowerCase()
  }
  return '#000000'
}
