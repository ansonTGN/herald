import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { ProgressIndicator, type Step } from '../progress-indicator'

describe('ProgressIndicator component', () => {
  const mockSteps: Step[] = [
    { id: 'step-1', title: 'Step 1' },
    { id: 'step-2', title: 'Step 2' },
    { id: 'step-3', title: 'Step 3' },
    { id: 'step-4', title: 'Step 4' },
  ]

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

  describe('connectors', () => {
    it('should render connectors between steps', () => {
      const { container } = render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')
      expect(connectors).toHaveLength(3)
    })

    it('should not render connector after last step', () => {
      const { container } = render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      const step4 = screen.getByTestId('progress-step-step-4')
      const nextSibling = step4.nextElementSibling

      expect(nextSibling?.getAttribute('data-slot')).not.toBe('progress-connector')
    })

    it('should mark connectors with appropriate status', () => {
      const { container } = render(<ProgressIndicator steps={mockSteps} currentStep={2} />)

      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')

      expect(connectors[0]).toHaveClass('bg-primary')
      expect(connectors[1]).toHaveClass('bg-primary')
      expect(connectors[2]).toHaveClass('bg-primary')
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

  describe('edge cases', () => {
    it('should handle single step', () => {
      const singleStep: Step[] = [{ id: 'step-1', title: 'Only Step' }]

      render(<ProgressIndicator steps={singleStep} currentStep={0} />)

      expect(screen.getByTestId('progress-step-step-1')).toBeInTheDocument()

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

      for (let i = 1; i <= 10; i++) {
        expect(screen.getByTestId(`progress-step-step-${i}`)).toBeInTheDocument()
      }

      const { container } = render(<ProgressIndicator steps={manySteps} currentStep={5} />)
      const connectors = container.querySelectorAll('[data-slot="progress-connector"]')
      expect(connectors).toHaveLength(9)
    })

    it('should handle currentStep at boundaries', () => {
      const { rerender } = render(<ProgressIndicator steps={mockSteps} currentStep={0} />)

      expect(screen.getByTestId('progress-step-step-1')).toHaveAttribute('data-status', 'current')

      rerender(<ProgressIndicator steps={mockSteps} currentStep={3} />)
      expect(screen.getByTestId('progress-step-step-4')).toHaveAttribute('data-status', 'current')
    })
  })
})
