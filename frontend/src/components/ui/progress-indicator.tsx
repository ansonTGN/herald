import * as React from 'react'
import { CheckIcon } from 'lucide-react'

import { cn } from '@/lib/utils'

// ==================== Progress Indicator ====================

export interface Step {
  id: string
  title?: string
}

type StepStatus = 'completed' | 'current' | 'pending'

interface ProgressIndicatorProps extends React.ComponentProps<'div'> {
  steps: Step[]
  currentStep: number
}

function ProgressIndicator({ steps, currentStep, className, ...props }: ProgressIndicatorProps) {
  return (
    <div
      data-slot="progress-indicator"
      data-testid="progress-indicator"
      className={cn('flex w-full items-center', className)}
      aria-label={`Step ${currentStep + 1} of ${steps.length}`}
      {...props}
    >
      {steps.map((step, index) => {
        const status: StepStatus =
          index < currentStep ? 'completed' : index === currentStep ? 'current' : 'pending'

        return (
          <React.Fragment key={step.id}>
            <StepItem step={step} index={index} status={status} />
            {index < steps.length - 1 && <StepConnector status={status} />}
          </React.Fragment>
        )
      })}
    </div>
  )
}

interface StepItemProps {
  step: Step
  index: number
  status: StepStatus
}

function StepItem({ step, index, status }: StepItemProps) {
  return (
    <div
      data-slot="progress-step"
      className="flex flex-col items-center gap-1.5"
      data-testid={`progress-step-${step.id}`}
      data-status={status}
    >
      {/* Circle indicator */}
      <div
        className={cn(
          'flex size-9 items-center justify-center rounded-full border-2 transition-colors duration-200',
          status === 'completed' && 'border-blue-500 bg-blue-500 text-white',
          status === 'current' && 'border-blue-500 bg-blue-500 text-white',
          status === 'pending' && 'border-muted-foreground/30 text-muted-foreground/50'
        )}
        aria-current={status === 'current' ? 'step' : undefined}
      >
        {status === 'completed' ? (
          <CheckIcon className="size-4" />
        ) : (
          <span className="text-sm font-medium">{index + 1}</span>
        )}
      </div>

      {/* Step title */}
      {step.title && (
        <span
          className={cn(
            'text-xs transition-colors duration-200',
            status === 'completed' && 'text-blue-600 font-medium',
            status === 'current' && 'text-blue-600 font-semibold',
            status === 'pending' && 'text-muted-foreground'
          )}
        >
          {step.title}
        </span>
      )}
    </div>
  )
}

interface StepConnectorProps {
  status: StepStatus
}

function StepConnector({ status }: StepConnectorProps) {
  return (
    <div
      data-slot="progress-connector"
      className={cn(
        'mx-2 h-0.5 flex-1 transition-colors duration-200',
        status === 'completed' && 'bg-blue-500',
        status === 'current' && 'bg-blue-500',
        status === 'pending' && 'bg-muted-foreground/30'
      )}
      aria-hidden="true"
    />
  )
}

export { ProgressIndicator }
