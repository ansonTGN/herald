import type { ReactNode } from 'react'
import { useCallback } from 'react'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { getFieldErrorMessage } from '@/lib/form-utils'

interface TextareaFieldProps {
  form: any // eslint-disable-line @typescript-eslint/no-explicit-any -- shared field wrapper accepts the repository's typed form instance
  name: string
  label: ReactNode
  inputId?: string
  dataTestId?: string
  placeholder?: string
  disabled?: boolean
  helpText?: ReactNode
  rows?: number
  required?: boolean
  onSubmit?: () => void
}

export function TextareaField({
  form,
  name,
  label,
  inputId,
  dataTestId,
  placeholder,
  disabled = false,
  helpText,
  rows = 3,
  required = false,
  onSubmit,
}: TextareaFieldProps) {
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && onSubmit) {
        e.preventDefault()
        onSubmit()
      }
    },
    [onSubmit]
  )

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
          <Textarea
            id={inputId ?? field.name}
            data-testid={dataTestId}
            value={field.state.value ?? ''}
            onBlur={field.handleBlur}
            onChange={(e) => field.handleChange(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={placeholder}
            disabled={disabled}
            rows={rows}
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
