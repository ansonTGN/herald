import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { getUserStatusOptions } from '@/lib/constants/user'
import { m } from '@/paraglide/messages'

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
        <SelectValue placeholder={m['users.form_select_status']()} />
      </SelectTrigger>
      <SelectContent>
        {getUserStatusOptions().map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
