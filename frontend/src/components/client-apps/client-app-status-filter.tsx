import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

const STATUS_OPTIONS = [
  { label: 'All', value: 'all' },
  { label: 'Enabled', value: 'true' },
  { label: 'Disabled', value: 'false' },
] as const

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

  return (
    <Select
      value={value === undefined ? 'all' : String(value)}
      onValueChange={handleValueChange}
      data-testid="client-app-status-filter"
    >
      <SelectTrigger className="w-[180px]">
        <SelectValue placeholder="Filter by status" />
      </SelectTrigger>
      <SelectContent>
        {STATUS_OPTIONS.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
