import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { Step4Review } from '../step-4-review'
import type { Step1FormData, Step2FormData, Step3FormData } from '..'

describe('Step4Review', () => {
  const mockOnEditStep = vi.fn()

  const validFormData: Partial<Step1FormData & Step2FormData & Step3FormData> = {
    name: 'Test App',
    description: 'Test Description',
    appType: 'WEB',
    clientType: 'CONFIDENTIAL',
    redirectUris: ['https://example.com/callback'],
    postLogoutUris: ['https://example.com/logout'],
    webOrigins: ['https://example.com'],
    sessionTtlSeconds: 3600,
    sessionRenewalTtlSeconds: 7200,
  }

  describe('Render - Create Mode', () => {
    it('should render review step with all sections', () => {
      render(<Step4Review mode="create" formData={validFormData} onEditStep={mockOnEditStep} />)

      expect(screen.getByTestId('review-step')).toBeInTheDocument()
      expect(screen.getByText(/Review & Create/i)).toBeInTheDocument()
      expect(screen.getByTestId('review-basic-info')).toBeInTheDocument()
      expect(screen.getByTestId('review-redirect-uris')).toBeInTheDocument()
      expect(screen.getByTestId('review-security')).toBeInTheDocument()
    })

    it('should display all form data correctly', () => {
      render(<Step4Review mode="create" formData={validFormData} onEditStep={mockOnEditStep} />)

      // Basic info
      expect(screen.getByText('Test App')).toBeInTheDocument()
      expect(screen.getByText('Test Description')).toBeInTheDocument()
      expect(screen.getByText('Web Application')).toBeInTheDocument()
      expect(screen.getByText('Confidential')).toBeInTheDocument()

      // Redirect URIs
      expect(screen.getByText('https://example.com/callback')).toBeInTheDocument()
      expect(screen.getByText('https://example.com/logout')).toBeInTheDocument()
      expect(screen.getByText('https://example.com')).toBeInTheDocument()

      // Security settings
      expect(screen.getByText(/1h 0m \(3600s\)/)).toBeInTheDocument()
      expect(screen.getByText(/2h 0m \(7200s\)/)).toBeInTheDocument()
    })

    it('should show complete badges for all valid sections', () => {
      render(<Step4Review mode="create" formData={validFormData} onEditStep={mockOnEditStep} />)

      const completeBadges = screen.getAllByText('Complete')
      expect(completeBadges).toHaveLength(3)
    })

    it('should not show validation warning when form is valid', () => {
      render(<Step4Review mode="create" formData={validFormData} onEditStep={mockOnEditStep} />)

      expect(screen.queryByText(/Please complete all required fields/i)).not.toBeInTheDocument()
    })
  })

  describe('Render - Edit Mode', () => {
    it('should show "Save Changes" instead of "Create"', () => {
      render(<Step4Review mode="edit" formData={validFormData} onEditStep={mockOnEditStep} />)

      expect(screen.getByText(/Review & Save Changes/i)).toBeInTheDocument()
    })

    it('should show appropriate security notice for edit mode', () => {
      render(<Step4Review mode="edit" formData={validFormData} onEditStep={mockOnEditStep} />)

      expect(screen.getByText(/Changes will take effect immediately/i)).toBeInTheDocument()
    })
  })

  describe('Validation States', () => {
    it('should show incomplete badge for missing basic info', () => {
      const incompleteData = { ...validFormData, name: '' }
      render(<Step4Review mode="create" formData={incompleteData} onEditStep={mockOnEditStep} />)

      expect(screen.getByText('Incomplete')).toBeInTheDocument()
    })

    it('should show incomplete badge for missing redirect URIs', () => {
      const incompleteData = { ...validFormData, redirectUris: [] }
      render(<Step4Review mode="create" formData={incompleteData} onEditStep={mockOnEditStep} />)

      expect(screen.getByText('Incomplete')).toBeInTheDocument()
    })

    it('should show validation warning with incomplete sections', () => {
      const incompleteData: Partial<Step1FormData & Step2FormData & Step3FormData> = {
        name: '',
        redirectUris: [],
        sessionTtlSeconds: 30, // Less than minimum
      }
      render(<Step4Review mode="create" formData={incompleteData} onEditStep={mockOnEditStep} />)

      // Check that validation warning is shown
      expect(screen.getByText(/Please complete all required fields/i)).toBeInTheDocument()

      // Check that incomplete sections are mentioned (use getAllByText as the text may be split across elements)
      const basicInfoTexts = screen.queryAllByText(/Basic Information/)
      expect(basicInfoTexts.length).toBeGreaterThan(0)
    })
  })

  describe('Edit Navigation', () => {
    it('should call onEditStep with correct step index when clicking edit buttons', async () => {
      const user = userEvent.setup()
      render(<Step4Review mode="create" formData={validFormData} onEditStep={mockOnEditStep} />)

      const editButtons = screen.getAllByRole('button', { name: /Edit/i })

      await user.click(editButtons[0]) // Step 0
      expect(mockOnEditStep).toHaveBeenCalledWith(0)

      await user.click(editButtons[1]) // Step 1
      expect(mockOnEditStep).toHaveBeenCalledWith(1)

      await user.click(editButtons[2]) // Step 2
      expect(mockOnEditStep).toHaveBeenCalledWith(2)
    })

    it('should render correct data-testid for edit buttons', () => {
      render(<Step4Review mode="create" formData={validFormData} onEditStep={mockOnEditStep} />)

      expect(screen.getByTestId('edit-step-0')).toBeInTheDocument()
      expect(screen.getByTestId('edit-step-1')).toBeInTheDocument()
      expect(screen.getByTestId('edit-step-2')).toBeInTheDocument()
    })
  })

  describe('Edge Cases', () => {
    it('should handle empty optional fields gracefully', () => {
      const minimalData: Partial<Step1FormData & Step2FormData & Step3FormData> = {
        name: 'Minimal App',
        appType: 'WEB',
        clientType: 'PUBLIC',
        redirectUris: ['https://example.com/callback'],
        sessionTtlSeconds: 3600,
      }
      render(<Step4Review mode="create" formData={minimalData} onEditStep={mockOnEditStep} />)

      expect(screen.getByText('-')).toBeInTheDocument() // Description
      expect(screen.getByText('Not configured')).toBeInTheDocument() // Renewal TTL
    })

    it('should handle large number of redirect URIs', () => {
      const manyUris = Array.from({ length: 10 }, (_, i) => `https://example${i}.com/callback`)
      const data = { ...validFormData, redirectUris: manyUris }
      render(<Step4Review mode="create" formData={data} onEditStep={mockOnEditStep} />)

      manyUris.forEach((uri) => {
        expect(screen.getByText(uri)).toBeInTheDocument()
      })
    })

    it('should handle submitting state', () => {
      render(
        <Step4Review
          mode="create"
          formData={validFormData}
          onEditStep={mockOnEditStep}
          isSubmitting
        />
      )

      expect(screen.getByText('Submitting...')).toBeInTheDocument()
    })
  })

  describe('Session Duration Formatting', () => {
    it('should format session duration correctly for hours', () => {
      const data = {
        ...validFormData,
        sessionTtlSeconds: 7200,
        sessionRenewalTtlSeconds: undefined,
      }
      render(<Step4Review mode="create" formData={data} onEditStep={mockOnEditStep} />)

      expect(screen.getByText(/2h 0m \(7200s\)/)).toBeInTheDocument()
    })

    it('should format session duration correctly for minutes only', () => {
      const data = {
        ...validFormData,
        sessionTtlSeconds: 1800,
        sessionRenewalTtlSeconds: undefined,
      }
      render(<Step4Review mode="create" formData={data} onEditStep={mockOnEditStep} />)

      expect(screen.getByText(/30m \(1800s\)/)).toBeInTheDocument()
    })
  })
})
