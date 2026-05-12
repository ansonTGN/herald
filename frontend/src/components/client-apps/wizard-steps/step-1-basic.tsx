import { Label } from '@/components/ui/label'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { TextField, TextareaField } from '@/components/shared/form-fields'
import { getFieldErrorMessage } from '@/lib/form-utils'
import type { ClientAppWizardMode } from '../client-app-wizard'
import { APP_TYPE_OPTIONS, CLIENT_TYPE_OPTIONS } from './step-1-schema'
import { useWizardFormContext } from '../wizard-form-context'

interface Step1BasicProps {
  mode: ClientAppWizardMode
}

export function Step1Basic({ mode }: Step1BasicProps) {
  console.log('[Step1Basic] Rendering', { mode })

  const { form, onNext } = useWizardFormContext()

  console.log('[Step1Basic] Got form from context', {
    hasForm: !!form,
    formValues: form?.state?.values,
    hasNext: !!onNext,
  })

  return (
    <div data-testid="basic-info-step" className="space-y-6">
      <div>
        <h2 className="text-lg font-semibold mb-2">Basic Information</h2>
        <p className="text-sm text-muted-foreground">
          {mode === 'create'
            ? 'Enter the basic details for your new client application.'
            : 'Edit the basic information for this client application.'}
        </p>
      </div>

      {/* App Name Field */}
      <TextField
        form={form}
        name="name"
        label="App Name"
        inputId="app-name"
        dataTestId="client-app-name-input"
        placeholder="My Awesome App"
        required
        onSubmit={onNext}
        helpText="Choose a descriptive name for your app (1-100 characters)"
      />

      {/* Description Field */}
      <TextareaField
        form={form}
        name="description"
        label="Description"
        inputId="app-description"
        dataTestId="client-app-description-input"
        placeholder="Optional description of your app's purpose"
        rows={3}
        onSubmit={onNext}
        helpText="Optional description of your app's purpose (max 500 characters)"
      />

      {/* App Type Field */}
      <form.Field
        name="appType"
        children={(
          field: any // eslint-disable-line @typescript-eslint/no-explicit-any -- generic field component
        ) => (
          <div className="space-y-2">
            <Label htmlFor="app-type">App Type *</Label>
            <Select
              value={field.state.value ?? ''}
              onValueChange={(value) => field.handleChange(value)}
            >
              <SelectTrigger
                id="app-type"
                aria-describedby="app-type-help"
                data-testid="client-app-app-type-select"
              >
                <SelectValue placeholder="Select app type" />
              </SelectTrigger>
              <SelectContent>
                {APP_TYPE_OPTIONS.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p id="app-type-help" className="text-xs text-muted-foreground">
              Select the type of your application
            </p>
            {(field.state.meta.isTouched || form.state.isSubmitted) &&
              field.state.meta.errors.length > 0 && (
                <p className="text-sm text-destructive" role="alert">
                  {getFieldErrorMessage(field.state.meta)}
                </p>
              )}
          </div>
        )}
      />

      {/* Client Type Field */}
      <form.Field
        name="clientType"
        children={(
          field: any // eslint-disable-line @typescript-eslint/no-explicit-any -- generic field component
        ) => (
          <div className="space-y-2">
            <Label>Client Type *</Label>
            <RadioGroup
              value={field.state.value ?? ''}
              onValueChange={(value) => field.handleChange(value)}
              className="flex flex-col gap-3"
              data-testid="client-app-client-type-radiogroup"
              aria-required="true"
            >
              {CLIENT_TYPE_OPTIONS.map((option) => (
                <div
                  key={option.value}
                  className="flex items-start space-x-2 border rounded-md p-3 hover:bg-accent/50 transition-colors"
                >
                  <RadioGroupItem
                    value={option.value}
                    id={`client-type-${option.value.toLowerCase()}`}
                    data-testid={`client-app-client-type-${option.value.toLowerCase()}-radio`}
                  />
                  <div className="flex-1 space-y-1">
                    <Label
                      htmlFor={`client-type-${option.value.toLowerCase()}`}
                      className="font-normal cursor-pointer"
                    >
                      {option.label}
                    </Label>
                    <p className="text-xs text-muted-foreground">{option.description}</p>
                  </div>
                </div>
              ))}
            </RadioGroup>
            {(field.state.meta.isTouched || form.state.isSubmitted) &&
              field.state.meta.errors.length > 0 && (
                <p className="text-sm text-destructive" role="alert">
                  {getFieldErrorMessage(field.state.meta)}
                </p>
              )}
          </div>
        )}
      />
    </div>
  )
}
