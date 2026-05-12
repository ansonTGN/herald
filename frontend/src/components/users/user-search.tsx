import { useState, useEffect } from 'react'
import { useDebounce } from '@/hooks/use-debounce'
import { Input } from '@/components/ui/input'

interface UserSearchProps {
  email?: string
  onSearchChange: (email: string | undefined) => void
}

export function UserSearch({ email = '', onSearchChange }: UserSearchProps) {
  const [searchInput, setSearchInput] = useState(email)
  const debouncedSearch = useDebounce(searchInput, 500)

  useEffect(() => {
    if (debouncedSearch !== email) {
      onSearchChange(debouncedSearch || undefined)
    }
  }, [debouncedSearch, email, onSearchChange])

  return (
    <Input
      placeholder="Search by email..."
      value={searchInput}
      onChange={(e) => setSearchInput(e.target.value)}
      data-testid="users-search-input"
    />
  )
}
