import { useState, useEffect } from 'react'
import { Input } from '@/components/ui/input'

interface ClientAppSearchProps {
  value: string
  onChange: (value: string) => void
  debounceMs?: number
}

export function ClientAppSearch({ value, onChange, debounceMs = 500 }: ClientAppSearchProps) {
  const [searchInput, setSearchInput] = useState(value)

  useEffect(() => {
    const timer = setTimeout(() => {
      onChange(searchInput)
    }, debounceMs)

    return () => clearTimeout(timer)
  }, [searchInput, debounceMs, onChange])

  return (
    <Input
      placeholder="Search by name or client ID..."
      value={searchInput}
      onChange={(e) => setSearchInput(e.target.value)}
      data-testid="client-app-search-input"
    />
  )
}
