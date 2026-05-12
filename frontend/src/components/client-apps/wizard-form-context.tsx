/* eslint-disable react-refresh/only-export-components */
import React, { createContext, useContext, useRef } from 'react'

/**
 * React Context for sharing the wizard form instance across components
 *
 * This context provides type-safe access to the form instance for all wizard steps.
 * It follows React best practices for context usage with forms.
 *
 * @example
 * ```tsx
 * // In wizard component
 * const form = useAppForm({
 *   schema: wizardSchema,
 *   defaultValues: mapInitialData(initialData),
 * })
 *
 * return (
 *   <WizardFormProvider form={form}>
 *     <Step1Basic />
 *     <Step2Redirects />
 *     <Step3Security />
 *   </WizardFormProvider>
 * )
 *
 * // In step components
 * function Step1Basic() {
 *   const form = useWizardFormContext()
 *   return (
 *     <form.Field
 *       name="name"
 *       children={(field) => <input value={field.state.value} onChange={(e) => field.handleChange(e.target.value)} />}
 *     />
 *   )
 * }
 * ```
 */

interface WizardFormContextValue {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  form: any // Form instance is typed by useAppForm
  onNext?: () => void // Optional function to trigger next step navigation
}

const WizardFormContext = createContext<WizardFormContextValue | null>(null)

export function WizardFormProvider({
  form,
  onNext,
  children,
}: {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  form: any // Form instance is typed by useAppForm
  onNext?: () => void
  children: React.ReactNode
}) {
  // Use refs to hold the latest values without triggering context re-renders.
  // The form instance is stable across renders (same object, mutating .state),
  // but onNext is recreated on every parent render. Putting it in useMemo deps
  // causes the context value to change on every render, which triggers all
  // consumers to re-render, which can create an infinite loop with TanStack Form.
  const formRef = useRef(form)
  formRef.current = form
  const onNextRef = useRef(onNext)
  onNextRef.current = onNext

  // Stable context value — never changes, so consumers don't re-render from this alone.
  // Consumers read the latest form/onNext via the refs at call time.
  const value = useRef({ form, onNext }).current
  value.form = form
  value.onNext = onNext

  return <WizardFormContext.Provider value={value}>{children}</WizardFormContext.Provider>
}

/**
 * Hook to access the wizard form context
 *
 * Provides type-safe access to the form instance from any child component.
 * Must be used within a WizardFormProvider.
 *
 * @throws {Error} If used outside of a WizardFormProvider
 * @returns The form instance from context
 *
 * @example
 * ```tsx
 * function Step1Basic() {
 *   const { form } = useWizardFormContext()
 *
 *   return (
 *     <form.Field
 *       name="name"
 *       children={(field) => (
 *         <input
 *           value={field.state.value}
 *           onChange={(e) => field.handleChange(e.target.value)}
 *         />
 *       )}
 *     />
 *   )
 * }
 * ```
 */
export function useWizardFormContext(): WizardFormContextValue {
  const context = useContext(WizardFormContext)

  if (!context) {
    throw new Error(
      'useWizardFormContext must be used within a WizardFormProvider. ' +
        'Wrap your component tree with <WizardFormProvider form={form}>'
    )
  }

  return context
}
