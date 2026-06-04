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
import { getFieldErrorMessage } from '@/lib/form-utils'
import { m } from '@/paraglide/messages'
import type { ResourceFormConfig } from './resource-form-config'

interface CreateResourceDialogProps<TData, TResponse = unknown> {
  open: boolean
  onOpenChange: (open: boolean) => void
  config: ResourceFormConfig<TData, TResponse>
  title: string
  description: ReactNode
  additionalFields?: ReactNode
}

export function CreateResourceDialog<TData, TResponse = unknown>({
  open,
  onOpenChange,
  config,
  title,
  description,
  additionalFields,
}: CreateResourceDialogProps<TData, TResponse>) {
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

  // Reset form when dialog opens
  useEffect(() => {
    if (open) {
      form.reset(config.defaultValues)
    }
  }, [config.defaultValues, form, open])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>{description}</DialogDescription>
        </DialogHeader>

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
                    placeholder={config.nameFieldPlaceholder}
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    data-testid={config.nameFieldTestId}
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-destructive">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                  {config.nameFieldHelpText && (
                    <p className="text-xs text-muted-foreground">{config.nameFieldHelpText}</p>
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
              <Button type="submit" disabled={isSubmitting} data-testid={config.submitButtonTestId}>
                {isSubmitting ? config.submittingButtonText : config.submitButtonText}
              </Button>
            </DialogFooter>
          </form>
        </AppForm>
      </DialogContent>
    </Dialog>
  )
}
