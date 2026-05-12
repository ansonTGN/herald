import type { ReactNode } from 'react'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { getFieldErrorMessage } from '@/lib/form-utils'

interface NumberFieldProps {
  form: any // eslint-disable-line @typescript-eslint/no-explicit-any -- shared field wrapper accepts the repository's typed form instance
  name: string
  label: ReactNode
  inputId?: string
  dataTestId?: string
  placeholder?: string
  step?: string
  disabled?: boolean
  helpText?: ReactNode
  min?: number
  max?: number
  required?: boolean
}

export function NumberField({
  form,
  name,
  label,
  inputId,
  dataTestId,
  placeholder,
  step,
  disabled = false,
  helpText,
  min,
  max,
  required = false,
}: NumberFieldProps) {
  return (
    <form.Field
      name={name}
      children={(
        field: any // eslint-disable-line @typescript-eslint/no-explicit-any -- generic field component
      ) => (
        <div className="space-y-2">
          <Label htmlFor={inputId ?? field.name}>
            {label}
            {required ? ' *' : null}
          </Label>
          <Input
            id={inputId ?? field.name}
            data-testid={dataTestId}
            type="number"
            value={field.state.value ?? ''}
            onBlur={field.handleBlur}
            onChange={(e) => field.handleChange(Number(e.target.value))}
            placeholder={placeholder}
            step={step}
            disabled={disabled}
            min={min}
            max={max}
          />
          {helpText ? <p className="text-xs text-muted-foreground">{helpText}</p> : null}
          {(field.state.meta.isTouched || form.state.isSubmitted) &&
            field.state.meta.errors.length > 0 && (
              <p className="text-sm text-destructive" role="alert">
                {getFieldErrorMessage(field.state.meta)}
              </p>
            )}
        </div>
      )}
    />
  )
}
