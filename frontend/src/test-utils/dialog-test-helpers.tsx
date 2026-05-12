import { expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'

type Screen = ReturnType<typeof render>

/**
 * Dialog test helpers - Common utilities for testing dialog components
 * Reduces code duplication across create/edit dialog tests
 */

/**
 * Asserts that dialog fields are not present when dialog is closed
 * @param testId - The data-testid of the input to check
 */
export async function assertDialogClosed(testId: string): Promise<void> {
  expect(screen.queryByTestId(testId)).toBeNull()
}

/**
 * Asserts that cancel button calls onOpenChange with false
 * @param screen - Vitest screen
 * @param handleOpenChange - Mock function to check
 */
export function assertCancelButtonWorks(
  screen: Screen,
  handleOpenChange: ReturnType<typeof vi.fn>
): void {
  const cancelButton = screen.getAllByText('Cancel', { exact: true })[0]
  cancelButton.click()

  expect(handleOpenChange).toHaveBeenCalledTimes(1)
  expect(handleOpenChange).toHaveBeenCalledWith(false)
}

/**
 * Asserts that typing in an input updates its value
 * @param screen - Vitest screen
 * @param testId - The data-testid of the input
 * @param value - The value to type
 * @param clearFirst - Whether to clear the input first (default: false)
 */
export async function assertInputUpdates(
  screen: Screen,
  testId: string,
  value: string,
  clearFirst = false
): Promise<void> {
  const input = screen.getByTestId(testId) as HTMLInputElement
  expect(input).not.toBeNull()

  if (clearFirst) {
    await userEvent.clear(input)
  }
  await userEvent.type(input, value)

  expect(input).toHaveValue(value)
}

/**
 * Asserts that dialog title and description are displayed
 * @param screen - Vitest screen
 * @param title - Expected dialog title
 * @param description - Expected dialog description (partial match supported)
 */
export function assertDialogTitleAndDescription(
  screen: Screen,
  title: string,
  description: string | RegExp
): void {
  expect(screen.getByText(title)).toBeInTheDocument()
  expect(screen.getByText(description)).toBeInTheDocument()
}

/**
 * Asserts that form fields are present and visible
 * @param screen - Vitest screen
 * @param fields - Array of field labels and their testids
 */
export function assertFormFieldsPresent(
  screen: Screen,
  fields: Array<{ label: string; testId: string }>
): void {
  for (const field of fields) {
    expect(screen.getAllByText(field.label, { exact: true })[0]).toBeInTheDocument()
    expect(screen.getByTestId(field.testId)).toBeInTheDocument()
  }
}

/**
 * Asserts that action buttons are present
 * @param screen - Vitest screen
 * @param buttons - Array of button labels to check
 */
export function assertButtonsPresent(screen: Screen, buttons: string[]): void {
  for (const button of buttons) {
    expect(screen.getAllByText(button, { exact: true })[0]).toBeInTheDocument()
  }
}

/**
 * Asserts that an input has a specific placeholder
 * @param screen - Vitest screen
 * @param testId - The data-testid of the input
 * @param placeholder - Expected placeholder text
 */
export function assertPlaceholder(screen: Screen, testId: string, placeholder: string): void {
  const input = screen.getByTestId(testId)
  expect(input).toHaveAttribute('placeholder', placeholder)
}

/**
 * Asserts that helper text is displayed
 * @param screen - Vitest screen
 * @param text - Helper text to check (partial match supported)
 */
export function assertHelperText(screen: Screen, text: string | RegExp): void {
  expect(screen.getByText(text)).toBeInTheDocument()
}

/**
 * Asserts that an element is disabled
 * @param screen - Vitest screen
 * @param testId - The data-testid of the element
 */
export function assertDisabled(screen: Screen, testId: string): void {
  const element = screen.getByTestId(testId)
  expect(element).toBeDisabled()
}

/**
 * Asserts that an element is enabled
 * @param screen - Vitest screen
 * @param testId - The data-testid of the element
 */
export function assertEnabled(screen: Screen, testId: string): void {
  const element = screen.getByTestId(testId)
  expect(element).toBeEnabled()
}

/**
 * Asserts that an input has a specific value
 * @param screen - Vitest screen
 * @param testId - The data-testid of the input
 * @param value - Expected value
 */
export function assertValue(screen: Screen, testId: string, value: string): void {
  const element = screen.getByTestId(testId)
  expect(element).toHaveValue(value)
}

/**
 * Type text into an input by testId
 * @param testId - The data-testid of the input
 * @param value - Value to type
 * @param clearFirst - Whether to clear input first (default: false)
 */
export async function typeInInput(
  testId: string,
  value: string,
  clearFirst = false
): Promise<void> {
  const input = document.querySelector(`[data-testid="${testId}"]`) as HTMLInputElement
  expect(input).not.toBeNull()

  if (clearFirst) {
    await userEvent.clear(input)
  }
  await userEvent.type(input, value)
}

/**
 * Click a button by testId
 * @param testId - The data-testid of the button
 */
export async function clickButton(testId: string): Promise<void> {
  const button = document.querySelector(`[data-testid="${testId}"]`) as HTMLElement
  expect(button).not.toBeNull()
  await userEvent.click(button)
}
