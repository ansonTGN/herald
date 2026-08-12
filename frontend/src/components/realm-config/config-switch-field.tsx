import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { getFieldErrorMessage } from '@/lib/form-utils'

interface ConfigSwitchFieldProps {
  field: {
    state: {
      value: boolean
      meta: {
        errors: unknown[]
        isTouched?: boolean
      }
    }
    handleChange: (value: boolean) => void
  }
  form?: {
    state: {
      isSubmitted: boolean
    }
  }
  id: string
  label: string
  description: string
  disabled?: boolean
  errorTestId?: string
  checked?: boolean
}

/**
 * Reusable switch field component for config forms.
 * Provides consistent layout and styling for boolean configuration options.
 */
export function ConfigSwitchField({
  field,
  form,
  id,
  label,
  description,
  disabled,
  errorTestId,
  checked,
}: ConfigSwitchFieldProps) {
  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <div className="space-y-0.5">
          <Label htmlFor={id}>{label}</Label>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>
        <Switch
          id={id}
          checked={checked ?? field.state.value}
          onCheckedChange={field.handleChange}
          disabled={disabled}
          data-testid={`${id}-switch`}
        />
      </div>
      {/* Form validation error display */}
      {(field.state.meta.isTouched || form?.state.isSubmitted) &&
        field.state.meta.errors.length > 0 && (
          <p className="text-sm text-destructive" data-testid={errorTestId}>
            {getFieldErrorMessage(field.state.meta)}
          </p>
        )}
    </div>
  )
}
