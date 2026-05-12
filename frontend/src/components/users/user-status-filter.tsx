import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { USER_STATUS_OPTIONS } from '@/lib/constants/user'

interface UserStatusFilterProps {
  onStatusChange: (status: number | undefined) => void
}

export function UserStatusFilter({ onStatusChange }: UserStatusFilterProps) {
  function handleValueChange(value: string) {
    const status = value === 'all' ? undefined : Number(value)
    onStatusChange(status)
  }

  return (
    <Select value="all" onValueChange={handleValueChange} data-testid="user-status-filter">
      <SelectTrigger className="w-[180px]">
        <SelectValue placeholder="Filter by status" />
      </SelectTrigger>
      <SelectContent>
        {USER_STATUS_OPTIONS.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
