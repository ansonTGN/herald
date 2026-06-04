import { useState, useCallback } from 'react'
import { Input } from '@/components/ui/input'
import { Button } from '@/components/ui/button'
import { filterAndFormat, toBackendCode, rawLength } from './device-code-utils'
import { m } from '@/paraglide/messages'

interface CodeInputProps {
  onSubmit: (userCode: string) => void
  defaultValue?: string
  isLoading?: boolean
}

export function CodeInput({ onSubmit, defaultValue, isLoading }: CodeInputProps) {
  const initialFormatted = defaultValue ? filterAndFormat(defaultValue) : ''
  const [value, setValue] = useState(initialFormatted)

  const isValid = rawLength(value) === 8

  const handleChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const formatted = filterAndFormat(e.target.value)
    setValue(formatted)
  }, [])

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault()
      if (!isValid) return
      onSubmit(toBackendCode(value))
    },
    [isValid, onSubmit, value]
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
        {m['device.verify_button']()}
      </Button>
    </form>
  )
}
