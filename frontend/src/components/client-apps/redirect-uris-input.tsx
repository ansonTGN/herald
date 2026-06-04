import { useState, useCallback } from 'react'
import { Plus, X, Check, AlertCircle } from 'lucide-react'
import { cn } from '@/lib/utils'
import { m } from '@/paraglide/messages'

export interface UriItem {
  id: string
  value: string
  isValid: boolean
}

interface RedirectUrisInputProps {
  value: UriItem[]
  onChange: (items: UriItem[]) => void
  placeholder?: string
  label?: string
  helpText?: string
  required?: boolean
  dataTestId?: string
  onSubmit?: () => void
}

/**
 * Validates if a string is a proper URL with http:// or https:// protocol
 */
function isValidUri(uri: string): boolean {
  if (!uri.trim()) return false
  try {
    const url = new URL(uri)
    return url.protocol === 'https:' || url.protocol === 'http:'
  } catch {
    return false
  }
}

/**
 * RedirectUrisInput - Dynamic URI list input component
 *
 * Features:
 * - Add/delete URIs with smooth animations
 * - Real-time URI format validation
 * - Visual status indicators (green=valid, red=invalid)
 * - Keyboard support (Enter to add, or trigger navigation when input is empty)
 * - Accessibility (ARIA labels, keyboard navigation)
 */
export function RedirectUrisInput({
  value,
  onChange,
  placeholder = 'https://example.com/callback',
  label,
  helpText,
  required = false,
  dataTestId = 'redirect-uris-input',
  onSubmit,
}: RedirectUrisInputProps) {
  const [inputValue, setInputValue] = useState('')
  const [errors, setErrors] = useState<Map<string, string>>(new Map())

  const handleAdd = useCallback(() => {
    const trimmed = inputValue.trim()
    if (!trimmed) return

    if (!isValidUri(trimmed)) {
      setErrors((prev) =>
        new Map(prev).set('new', m['client_apps.form_redirect_uris_invalid_url']())
      )
      return
    }

    // Check for duplicates
    if (value.some((item) => item.value === trimmed)) {
      setErrors((prev) => new Map(prev).set('new', m['client_apps.form_redirect_uris_duplicate']()))
      return
    }

    const newItem: UriItem = {
      id: `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      value: trimmed,
      isValid: true,
    }

    onChange([...value, newItem])
    setInputValue('')
    setErrors((prev) => {
      const next = new Map(prev)
      next.delete('new')
      return next
    })
  }, [inputValue, value, onChange])

  const handleRemove = useCallback(
    (id: string) => {
      onChange(value.filter((item) => item.id !== id))
    },
    [value, onChange]
  )

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter') {
        e.preventDefault()

        // If there's valid input content, add the URI first
        if (inputValue.trim() && isValidUri(inputValue)) {
          handleAdd()
        } else {
          // If input is empty or invalid, trigger navigation (e.g., move to next wizard step)
          onSubmit?.()
        }
      }
    },
    [handleAdd, onSubmit, inputValue]
  )

  const currentInputValid = inputValue.trim() !== '' && isValidUri(inputValue)

  return (
    <div className="space-y-3" data-testid={dataTestId}>
      {label && (
        <label className="text-sm font-medium">
          {label}
          {required && <span className="text-destructive ml-1">*</span>}
        </label>
      )}

      {/* Current URI list */}
      {value.length > 0 && (
        <div className="space-y-2">
          {value.map((item, index) => (
            <div
              key={item.id}
              className="uri-item flex items-center gap-2 p-3 bg-muted rounded-lg border border-border group animate-slide-in"
              style={{ animationDelay: `${index * 0.05}s` }}
              data-testid={`uri-item-${item.id}`}
            >
              <div className="flex-1 flex items-center gap-2">
                <div
                  className={cn(
                    'status-indicator w-2 h-2 rounded-full flex-shrink-0',
                    item.isValid ? 'bg-green-500' : 'bg-red-500'
                  )}
                  aria-hidden="true"
                />
                <code className="text-sm font-mono text-foreground break-all">{item.value}</code>
              </div>

              <button
                type="button"
                onClick={() => handleRemove(item.id)}
                className="opacity-0 group-hover:opacity-100 p-1.5 hover:bg-accent rounded transition-all focus:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring"
                aria-label={`Remove URI: ${item.value}`}
                data-testid={`remove-uri-${item.id}`}
              >
                <X className="w-4 h-4 text-muted-foreground" />
              </button>
            </div>
          ))}
        </div>
      )}

      {/* Add new URI input */}
      <div className="space-y-2">
        <div className="flex gap-2">
          <div className="flex-1 relative">
            <input
              type="text"
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder={placeholder}
              className={cn(
                'w-full px-4 py-2.5 rounded-lg border border-input bg-background transition-all',
                'focus:outline-none focus:ring-2 focus:ring-ring focus:border-transparent',
                'placeholder:text-muted-foreground',
                errors.get('new') && 'border-destructive focus:ring-destructive'
              )}
              aria-invalid={errors.get('new') !== undefined}
              aria-describedby={
                errors.get('new')
                  ? `${dataTestId}-error`
                  : helpText
                    ? `${dataTestId}-help`
                    : undefined
              }
              data-testid={`${dataTestId}-field`}
            />

            {/* Real-time validation feedback */}
            {inputValue && (
              <div
                className={cn(
                  'absolute right-3 top-1/2 -translate-y-1/2 transition-colors',
                  currentInputValid ? 'text-green-500' : 'text-red-500'
                )}
                aria-hidden="true"
              >
                {currentInputValid ? <Check className="w-5 h-5" /> : <X className="w-5 h-5" />}
              </div>
            )}
          </div>

          <button
            type="button"
            onClick={handleAdd}
            disabled={!inputValue.trim() || !currentInputValid}
            className={cn(
              'px-4 py-2.5 rounded-lg font-medium transition-all flex items-center gap-2',
              'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
              inputValue.trim() && currentInputValid
                ? 'bg-primary text-primary-foreground hover:bg-primary/90 active:scale-[0.98]'
                : 'bg-muted text-muted-foreground cursor-not-allowed'
            )}
            aria-label={m['client_apps.form_redirect_uris_add']()}
            data-testid={`${dataTestId}-add-button`}
          >
            <Plus className="w-4 h-4" />
            {m['client_apps.form_redirect_uris_add']()}
          </button>
        </div>

        {/* Error message */}
        {errors.get('new') && (
          <p
            id={`${dataTestId}-error`}
            className="text-sm text-destructive flex items-center gap-1 animate-slide-down"
            role="alert"
          >
            <AlertCircle className="w-4 h-4" />
            {errors.get('new')}
          </p>
        )}

        {/* Help text */}
        {helpText && !errors.get('new') && (
          <p id={`${dataTestId}-help`} className="text-xs text-muted-foreground">
            {helpText}
          </p>
        )}
      </div>
    </div>
  )
}
