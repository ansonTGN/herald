import { useState, useEffect } from 'react'
import { useDebounce } from '@/hooks/use-debounce'
import { Input } from '@/components/ui/input'

interface RealmSearchProps {
  realmId?: string
  onSearchChange: (realmId: string | undefined) => void
}

export function RealmSearch({ realmId = '', onSearchChange }: RealmSearchProps) {
  const [searchInput, setSearchInput] = useState(realmId)
  const debouncedSearch = useDebounce(searchInput, 500)

  useEffect(() => {
    if (debouncedSearch !== realmId) {
      onSearchChange(debouncedSearch || undefined)
    }
  }, [debouncedSearch, realmId, onSearchChange])

  return (
    <Input
      placeholder="Search by realm ID..."
      value={searchInput}
      onChange={(e) => setSearchInput(e.target.value)}
      data-testid="realms-search-input"
    />
  )
}
