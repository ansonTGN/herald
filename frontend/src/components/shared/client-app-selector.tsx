import { useState } from 'react'
import { Check, ChevronsUpDown } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { m } from '@/paraglide/messages'

interface ClientAppOption {
  id: string
  name: string
  clientId: string
}

interface ClientAppSelectorProps {
  clientApps: ClientAppOption[]
  value?: string
  onChange: (value: string | undefined) => void
  disabled?: boolean
}

export function ClientAppSelector({
  clientApps,
  value,
  onChange,
  disabled = false,
}: ClientAppSelectorProps) {
  const [open, setOpen] = useState(false)
  const selected = clientApps.find((app) => app.id === value)

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          disabled={disabled}
          className="w-full justify-between"
          data-testid="client-app-selector-trigger"
        >
          <span className={cn('truncate', !selected && 'text-muted-foreground')}>
            {selected ? `${selected.name} (${selected.clientId})` : m['shared.default_client']()}
          </span>
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-full p-0" align="start">
        <Command>
          <CommandInput
            placeholder={m['shared.search_client_apps']()}
            data-testid="client-app-selector-search"
          />
          <CommandList>
            <CommandEmpty>{m['shared.no_client_apps_found']()}</CommandEmpty>
            <CommandGroup>
              <CommandItem
                value="__default__"
                onSelect={() => {
                  onChange(undefined)
                  setOpen(false)
                }}
                data-testid="client-app-selector-default"
              >
                <Check className={cn('mr-2 h-4 w-4', !value ? 'opacity-100' : 'opacity-0')} />
                {m['shared.default_client']()}
              </CommandItem>
              {clientApps.map((app) => (
                <CommandItem
                  key={app.id}
                  value={`${app.name} ${app.clientId}`}
                  onSelect={() => {
                    onChange(app.id)
                    setOpen(false)
                  }}
                  data-testid={`client-app-selector-item-${app.id}`}
                >
                  <Check
                    className={cn('mr-2 h-4 w-4', value === app.id ? 'opacity-100' : 'opacity-0')}
                  />
                  <span className="truncate">
                    {app.name} ({app.clientId})
                  </span>
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
