import React, { type ReactNode } from 'react'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { getFieldErrorMessage } from '@/lib/form-utils'

interface PasswordFieldProps {
  form: any // eslint-disable-line @typescript-eslint/no-explicit-any -- shared field wrapper accepts the repository's typed form instance
  name: string
  label: ReactNode
  inputId?: string
  dataTestId?: string
  placeholder?: string
  disabled?: boolean
  helpText?: ReactNode
  required?: boolean
}

export function PasswordField({
  form,
  name,
  label,
  inputId,
  dataTestId,
  placeholder,
  disabled = false,
  helpText,
  required = false,
}: PasswordFieldProps) {
  const [isVisible, setIsVisible] = React.useState(false)

  const toggleVisibility = React.useCallback(() => setIsVisible((prev) => !prev), [])

  return (
    <form.Field
      name={name}
      children={(
        field: any // eslint-disable-line @typescript-eslint/no-explicit-any -- generic field component
      ) => {
        return (
          <div className="space-y-2">
            <Label htmlFor={inputId ?? field.name}>
              {label}
              {required ? ' *' : null}
            </Label>
            <div className="flex gap-2">
              <Input
                id={inputId ?? field.name}
                data-testid={dataTestId}
                type={isVisible ? 'text' : 'password'}
                value={field.state.value ?? ''}
                onBlur={field.handleBlur}
                onChange={(e) => field.handleChange(e.target.value)}
                placeholder={placeholder}
                disabled={disabled}
                className="flex-1"
              />
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={toggleVisibility}
                data-testid={`${dataTestId}-visibility-toggle`}
                className="shrink-0"
              >
                {isVisible ? 'Hide' : 'Show'}
              </Button>
            </div>
            {helpText ? <p className="text-xs text-muted-foreground">{helpText}</p> : null}
            {(field.state.meta.isTouched || form.state.isSubmitted) &&
              field.state.meta.errors.length > 0 && (
                <p className="text-sm text-destructive" role="alert">
                  {getFieldErrorMessage(field.state.meta)}
                </p>
              )}
          </div>
        )
      }}
    />
  )
}
