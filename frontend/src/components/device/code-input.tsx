import { useState, useCallback } from 'react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'

const VALID_CHARS = new Set('BCDFGHJKMNPQRSTVWXYZ')

function filterAndFormat(value: string): string {
  // Uppercase, keep only valid chars, limit to 8
  const upper = value.toUpperCase()
  const filtered = upper.split('').filter((c) => VALID_CHARS.has(c)).slice(0, 8).join('')
  // Insert hyphen after 4th char
  if (filtered.length <= 4) return filtered
  return filtered.slice(0, 4) + '-' + filtered.slice(4)
}

function extractRawCode(formatted: string): string {
  return formatted.replace('-', '')
}

interface CodeInputProps {
  onSubmit: (userCode: string) => void
  defaultValue?: string
  isLoading?: boolean
}

export function CodeInput({ onSubmit, defaultValue, isLoading }: CodeInputProps) {
  const initialFormatted = defaultValue ? filterAndFormat(defaultValue) : ''
  const [value, setValue] = useState(initialFormatted)

  const rawCode = extractRawCode(value)
  const isValid = rawCode.length === 8

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const formatted = filterAndFormat(e.target.value)
    setValue(formatted)
  }, [])

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault()
      if (!isValid) return
      onSubmit(rawCode)
    },
    [isValid, onSubmit, rawCode]
  )

  return (
    <form onSubmit={handleSubmit} className="space-y-4">
      <Input
        value={value}
        onChange={handleChange}
        placeholder="XXXX-XXXX"
        className="text-center text-2xl tracking-[0.3em] font-mono h-12"
        data-testid="device-code-input"
        autoComplete="off"
        autoFocus
      />
      <Button
        type="submit"
        disabled={!isValid || isLoading}
        loading={isLoading}
        className="w-full"
        data-testid="device-code-submit"
      >
        Verify
      </Button>
    </form>
  )
}
