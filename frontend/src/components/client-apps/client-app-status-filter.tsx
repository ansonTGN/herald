import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { m } from '@/paraglide/messages'

interface ClientAppStatusFilterProps {
  value: boolean | undefined
  onChange: (value: boolean | undefined) => void
}

export function ClientAppStatusFilter({ value, onChange }: ClientAppStatusFilterProps) {
  const handleValueChange = (newValue: string) => {
    if (newValue === 'all') {
      onChange(undefined)
    } else {
      onChange(newValue === 'true')
    }
  }

  const statusOptions = [
    { label: m['client_apps.filter_all'](), value: 'all' },
    { label: m['client_apps.filter_enabled'](), value: 'true' },
    { label: m['client_apps.filter_disabled'](), value: 'false' },
  ]

  return (
    <Select
      value={value === undefined ? 'all' : String(value)}
      onValueChange={handleValueChange}
      data-testid="client-app-status-filter"
    >
      <SelectTrigger className="w-[180px]">
        <SelectValue placeholder={m['client_apps.filter_placeholder']()} />
      </SelectTrigger>
      <SelectContent>
        {statusOptions.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
