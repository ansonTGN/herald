import type { ReactNode } from 'react'
import { Switch } from '@/components/ui/switch'
import { Label } from '@/components/ui/label'

interface SwitchFieldProps {
  form: any // eslint-disable-line @typescript-eslint/no-explicit-any -- shared field wrapper accepts the repository's typed form instance
  name: string
  label: ReactNode
  inputId?: string
  dataTestId?: string
  description?: ReactNode
}

export function SwitchField({
  form,
  name,
  label,
  inputId,
  dataTestId,
  description,
}: SwitchFieldProps) {
  return (
    <form.Field
      name={name}
      children={(
        field: any // eslint-disable-line @typescript-eslint/no-explicit-any -- generic field component
      ) => (
        <div className="flex items-center justify-between">
          <div className="space-y-0.5">
            <Label htmlFor={inputId ?? field.name}>{label}</Label>
            {description ? <p className="text-sm text-muted-foreground">{description}</p> : null}
          </div>
          <Switch
            id={inputId ?? field.name}
            checked={field.state.value}
            onCheckedChange={(checked: boolean) => field.handleChange(checked)}
            data-testid={dataTestId}
          />
        </div>
      )}
    />
  )
}
