import { useState, useEffect } from 'react'
import { useDebounce } from '@/hooks/use-debounce'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { getUserStatusOptions } from '@/lib/constants/user'
import { m } from '@/paraglide/messages'

interface UserSearchProps {
  email?: string
  status?: string
  onSearchChange: (email: string | undefined) => void
  onStatusChange?: (status: string | undefined) => void
}

export function UserSearch({
  email = '',
  status,
  onSearchChange,
  onStatusChange,
}: UserSearchProps) {
  const [searchInput, setSearchInput] = useState(email)
  const debouncedSearch = useDebounce(searchInput, 500)

  useEffect(() => {
    if (debouncedSearch !== email) {
      onSearchChange(debouncedSearch || undefined)
    }
  }, [debouncedSearch, email, onSearchChange])

  return (
    <>
      <Input
        placeholder={m['users.search_placeholder']()}
        value={searchInput}
        onChange={(e) => setSearchInput(e.target.value)}
        data-testid="users-search-input"
        className="max-w-xs"
      />

      {onStatusChange && (
        <Select
          value={status ?? 'all'}
          onValueChange={(value) => onStatusChange(value === 'all' ? undefined : value)}
        >
          <SelectTrigger data-testid="users-status-filter" className="w-[160px]">
            <SelectValue placeholder={m['users.filter_all_statuses']()} />
          </SelectTrigger>
          <SelectContent>
            {getUserStatusOptions().map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      )}
    </>
  )
}
