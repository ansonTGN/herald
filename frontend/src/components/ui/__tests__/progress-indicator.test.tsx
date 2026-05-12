import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ProgressIndicator, type Step } from '../progress-indicator'

describe('ProgressIndicator component', () => {
  const mockSteps: Step[] = [
    { id: 'step-1', title: 'Step 1' },
    { id: 'step-2', title: 'Step 2' },
    { id: 'step-3', title: 'Step 3' },
    { id: 'step-4', title: 'Step 4' },
  ]

  describe('rendering', () => {
    it('should render with data-testid attribute', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const indicator = screen.getByTestId('progress-indicator')
      expect(indicator).toBeInTheDocument()
    })

    it('should render all step items', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      expect(screen.getByTestId('progress-step-step-1')).toBeInTheDocument()
      expect(screen.getByTestId('progress-step-step-2')).toBeInTheDocument()
      expect(screen.getByTestId('progress-step-step-3')).toBeInTheDocument()
      expect(screen.getByTestId('progress-step-step-4')).toBeInTheDocument()
    })

    it('should render step titles when provided', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      expect(screen.getByText('Step 1')).toBeInTheDocument()
      expect(screen.getByText('Step 2')).toBeInTheDocument()
      expect(screen.getByText('Step 3')).toBeInTheDocument()
      expect(screen.getByText('Step 4')).toBeInTheDocument()
    })

    it('should not render title when not provided', () => {
      const stepsWithoutTitles: Step[] = [{ id: 'step-1' }, { id: 'step-2' }, { id: 'step-3' }]

      render(<ProgressIndicator steps={stepsWithoutTitles} currentStep={0} />)

      // Should still render steps, just without titles
      expect(screen.getByTestId('progress-step-step-1')).toBeInTheDocument()
      expect(screen.getByTestId('progress-step-step-2')).toBeInTheDocument()
      expect(screen.getByTestId('progress-step-step-3')).toBeInTheDocument()
    })

    it('should render step numbers for non-completed steps', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      expect(screen.getByText('1')).toBeInTheDocument()
      expect(screen.getByText('2')).toBeInTheDocument()
      expect(screen.getByText('3')).toBeInTheDocument()
      expect(screen.getByText('4')).toBeInTheDocument()
    })
  })

  describe('step status', () => {
    it('should mark first step as current when currentStep is 0', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const step1 = screen.getByTestId('progress-step-step-1')
      expect(step1).toHaveAttribute('data-status', 'current')
    })

    it('should mark previous steps as completed', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={2} />)

      const step1 = screen.getByTestId('progress-step-step-1')
      const step2 = screen.getByTestId('progress-step-step-2')
      const step3 = screen.getByTestId('progress-step-step-3')
      const step4 = screen.getByTestId('progress-step-step-4')

      expect(step1).toHaveAttribute('data-status', 'completed')
      expect(step2).toHaveAttribute('data-status', 'completed')
      expect(step3).toHaveAttribute('data-status', 'current')
      expect(step4).toHaveAttribute('data-status', 'pending')
    })

    it('should mark all steps as completed when on last step', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={3} />)

      const step1 = screen.getByTestId('progress-step-step-1')
      const step2 = screen.getByTestId('progress-step-step-2')
      const step3 = screen.getByTestId('progress-step-step-3')
      const step4 = screen.getByTestId('progress-step-step-4')

      expect(step1).toHaveAttribute('data-status', 'completed')
      expect(step2).toHaveAttribute('data-status', 'completed')
      expect(step3).toHaveAttribute('data-status', 'completed')
      expect(step4).toHaveAttribute('data-status', 'current')
    })

    it('should mark all steps as pending when currentStep is 0', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const step2 = screen.getByTestId('progress-step-step-2')
      const step3 = screen.getByTestId('progress-step-step-3')
      const step4 = screen.getByTestId('progress-step-step-4')

      expect(step2).toHaveAttribute('data-status', 'pending')
      expect(step3).toHaveAttribute('data-status', 'pending')
      expect(step4).toHaveAttribute('data-status', 'pending')
    })
  })

  describe('visual indicators', () => {
    it('should show checkmark for completed steps', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={2} />)

      const step1 = screen.getByTestId('progress-step-step-1')
      const step2 = screen.getByTestId('progress-step-step-2')

      // Completed steps should have checkmark icon (lucide-react CheckIcon)
      expect(step1.innerHTML).toContain('svg')
      expect(step2.innerHTML).toContain('svg')
    })

    it('should show number for current step', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={1} />)

      const step2 = screen.getByTestId('progress-step-step-2')
      expect(step2).toHaveAttribute('data-status', 'current')
      expect(step2.textContent).toContain('2')
    })

    it('should show number for pending steps', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const step2 = screen.getByTestId('progress-step-step-2')
      expect(step2).toHaveAttribute('data-status', 'pending')
      expect(step2.textContent).toContain('2')
    })
  })

  describe('connectors', () => {
    it('should render connectors between steps', () => {
      const { container } = render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      // Should have 3 connectors for 4 steps
      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')
      expect(connectors).toHaveLength(3)
    })

    it('should not render connector after last step', () => {
      const { container } = render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const step4 = screen.getByTestId('progress-step-step-4')
      const nextSibling = step4.nextElementSibling

      // Last step should not have a connector after it
      expect(nextSibling?.getAttribute('data-slot')).not.toBe('progress-connector')
    })

    it('should mark connectors with appropriate status', () => {
      const { container } = render(<ProgressIndicator steps={mockSteps} currentStep={2} />)

      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')

      // First two connectors should be completed (connecting completed steps)
      expect(connectors[0]).toHaveClass('bg-blue-500')
      expect(connectors[1]).toHaveClass('bg-blue-500')

      // Third connector should be current (between current and next)
      expect(connectors[2]).toHaveClass('bg-blue-500')
    })
  })

  describe('accessibility', () => {
    it('should have proper aria-label', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const indicator = screen.getByTestId('progress-indicator')
      expect(indicator).toHaveAttribute('aria-label', 'Step 1 of 4')
    })

    it('should update aria-label based on current step', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={2} />)

      const indicator = screen.getByTestId('progress-indicator')
      expect(indicator).toHaveAttribute('aria-label', 'Step 3 of 4')
    })

    it('should mark current step with aria-current', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={1} />)

      const step2 = screen.getByTestId('progress-step-step-2')
      const circle = step2.querySelector('[aria-current="step"]')

      expect(circle).toBeInTheDocument()
    })

    it('should not have aria-current on non-current steps', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={1} />)

      const step1 = screen.getByTestId('progress-step-step-1')
      const step3 = screen.getByTestId('progress-step-step-3')

      expect(step1.querySelector('[aria-current]')).not.toBeInTheDocument()
      expect(step3.querySelector('[aria-current]')).not.toBeInTheDocument()
    })

    it('should hide connectors from screen readers', () => {
      const { container } = render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')

      connectors.forEach((connector) => {
        expect(connector).toHaveAttribute('aria-hidden', 'true')
      })
    })
  })

  describe('data attributes', () => {
    it('should pass through custom data attributes', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} data-custom="value" />)

      const indicator = screen.getByTestId('progress-indicator')
      expect(indicator.getAttribute('data-custom')).toBe('value')
    })

    it('should pass through className', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} className="custom-class" />)

      const indicator = screen.getByTestId('progress-indicator')
      expect(indicator).toHaveClass('custom-class')
    })
  })

  describe('edge cases', () => {
    it('should handle single step', () => {
      const singleStep: Step[] = [{ id: 'step-1', title: 'Only Step' }]

      render(<ProgressIndicator steps={singleStep} currentStep={0} />)

      expect(screen.getByTestId('progress-step-step-1')).toBeInTheDocument()
      expect(screen.getByText('Only Step')).toBeInTheDocument()

      // Should not have any connectors
      const { container } = render(<ProgressIndicator steps={singleStep} currentStep={0} />)
      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')
      expect(connectors).toHaveLength(0)
    })

    it('should handle two steps', () => {
      const twoSteps: Step[] = [
        { id: 'step-1', title: 'First' },
        { id: 'step-2', title: 'Second' },
      ]

      render(<ProgressIndicator steps={twoSteps} currentStep={0} />)

      expect(screen.getByTestId('progress-step-step-1')).toBeInTheDocument()
      expect(screen.getByTestId('progress-step-step-2')).toBeInTheDocument()

      // Should have one connector
      const { container } = render(<ProgressIndicator steps={twoSteps} currentStep={0} />)
      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')
      expect(connectors).toHaveLength(1)
    })

    it('should handle many steps', () => {
      const manySteps: Step[] = Array.from({ length: 10 }, (_, i) => ({
        id: `step-${i + 1}`,
        title: `Step ${i + 1}`,
      }))

      render(<ProgressIndicator steps={manySteps} currentStep={5} />)

      // Should render all 10 steps
      for (let i = 1; i <= 10; i++) {
        expect(screen.getByTestId(`progress-step-step-${i}`)).toBeInTheDocument()
      }

      // Should have 9 connectors
      const { container } = render(<ProgressIndicator steps={manySteps} currentStep={5} />)
      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')
      expect(connectors).toHaveLength(9)
    })

    it('should handle currentStep at boundaries', () => {
      const { rerender } = render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      // First step
      expect(screen.getByTestId('progress-step-step-1')).toHaveAttribute('data-status', 'current')

      // Last step
      rerender(<ProgressIndicator steps={mockSteps} currentStep={3} />)
      expect(screen.getByTestId('progress-step-step-4')).toHaveAttribute('data-status', 'current')
    })
  })

  describe('styling and layout', () => {
    it('should apply flex layout to container', () => {
      render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const indicator = screen.getByTestId('progress-indicator')
      expect(indicator).toHaveClass('flex', 'w-full', 'items-center')
    })

    it('should render steps in correct order', () => {
      const { container } = render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const steps = container.querySelectorAll('[data-slot="progress-step"]')
      expect(steps).toHaveLength(4)

      expect(steps[0].getAttribute('data-testid')).toBe('progress-step-step-1')
      expect(steps[1].getAttribute('data-testid')).toBe('progress-step-step-2')
      expect(steps[2].getAttribute('data-testid')).toBe('progress-step-step-3')
      expect(steps[3].getAttribute('data-testid')).toBe('progress-step-step-4')
    })
  })

  describe('props forwarding', () => {
    it('should forward additional div props to container', () => {
      render(
        <ProgressIndicator
          steps={mockSteps}
          currentStep={0}
          role="navigation"
          aria-label="Progress"
        />
      )

      const indicator = screen.getByTestId('progress-indicator')
      expect(indicator).toHaveAttribute('role', 'navigation')
    })
  })
})
