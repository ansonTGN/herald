import { useEffect } from 'react'
import type { ReactNode } from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { useFormMutation } from '@/hooks/use-form-mutation'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { AlertCircle } from 'lucide-react'
import { m } from '@/paraglide/messages'
import type { ResourceFormConfig } from './resource-form-config'

interface BuiltinProtectionConfig {
  isBuiltin: boolean
  alertMessage: string
  disabledFieldHelpText: string
}

interface EditResourceDialogProps<TData, TResponse = unknown> {
  open: boolean
  onOpenChange: (open: boolean) => void
  config: ResourceFormConfig<TData, TResponse>
  title: string
  description: string
  builtinProtection?: BuiltinProtectionConfig
  currentValues: TData
  additionalFields?: ReactNode
}

export function EditResourceDialog<TData, TResponse = unknown>({
  open,
  onOpenChange,
  config,
  title,
  description,
  builtinProtection,
  currentValues,
  additionalFields,
}: EditResourceDialogProps<TData, TResponse>) {
  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: config.mutationFn,
    getSuccessMessage: config.getSuccessMessage,
    invalidateQueries: config.queryKeysToInvalidate,
    onSuccess: () => {
      onOpenChange(false)
    },
  })

  const form = useAppForm({
    schema: config.schema,
    defaultValues: config.defaultValues,
    onSubmit: async ({ value }) => {
      await mutate(value as TData)
    },
  })

  // Update form values when resource changes
  useEffect(() => {
    if (currentValues) {
      form.reset(currentValues)
    }
  }, [currentValues, form])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

        {builtinProtection?.isBuiltin && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{builtinProtection.alertMessage}</AlertDescription>
          </Alert>
        )}

        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              e.stopPropagation()
              form.handleSubmit()
            }}
            className="space-y-4"
          >
            <form.Field
              name="name"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor={config.nameInputId}>{config.nameFieldLabel}</Label>
                  <Input
                    id={config.nameInputId}
                    type="text"
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    disabled={builtinProtection?.isBuiltin}
                    data-testid={config.nameFieldTestId}
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                  {builtinProtection?.isBuiltin && (
                    <p className="text-xs text-muted-foreground">
                      {builtinProtection.disabledFieldHelpText}
                    </p>
                  )}
                </div>
              )}
            />

            <form.Field
              name="description"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor={config.descriptionInputId}>{m['common.description']()}</Label>
                  <Textarea
                    id={config.descriptionInputId}
                    placeholder={config.descriptionFieldPlaceholder}
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    rows={3}
                    data-testid={config.descriptionFieldTestId}
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            />

            {additionalFields}

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
                disabled={isSubmitting}
              >
                {m['common.cancel']()}
              </Button>
              <Button
                type="submit"
                disabled={isSubmitting || builtinProtection?.isBuiltin}
                data-testid={config.submitButtonTestId}
              >
                {isSubmitting ? config.submittingButtonText : config.submitButtonText}
              </Button>
            </DialogFooter>
          </form>
        </AppForm>
      </DialogContent>
    </Dialog>
  )
}
