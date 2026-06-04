import { useState, useEffect } from 'react'
import { Input } from '@/components/ui/input'
import { m } from '@/paraglide/messages'

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
      placeholder={m['client_apps.search_placeholder']()}
      value={searchInput}
      onChange={(e) => setSearchInput(e.target.value)}
      data-testid="client-app-search-input"
    />
  )
}
