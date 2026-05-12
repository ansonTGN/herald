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
import { Badge } from '@/components/ui/badge'

interface Role {
  id: string
  name: string
}

interface RoleSelectorProps {
  roles: Role[]
  selectedRoleIds: string[]
  onChange: (roleIds: string[]) => void
  disabled?: boolean
  placeholder?: string
}

export function RoleSelector({
  roles,
  selectedRoleIds,
  onChange,
  disabled = false,
  placeholder = 'Select roles',
}: RoleSelectorProps) {
  const [open, setOpen] = useState(false)

  const selectedRoles = roles.filter((role) => selectedRoleIds.includes(role.id))

  const handleToggleRole = (roleId: string) => {
    if (selectedRoleIds.includes(roleId)) {
      onChange(selectedRoleIds.filter((id) => id !== roleId))
    } else {
      onChange([...selectedRoleIds, roleId])
    }
  }

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          role="combobox"
          aria-expanded={open}
          disabled={disabled}
          className="w-full justify-between"
          data-testid="role-selector-trigger"
        >
          {selectedRoles.length > 0 ? (
            <div className="flex flex-wrap gap-1">
              {selectedRoles.map((role) => (
                <Badge key={role.id} variant="secondary">
                  {role.name}
                </Badge>
              ))}
            </div>
          ) : (
            <span className="text-muted-foreground">{placeholder}</span>
          )}
          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-full p-0" align="start">
        <Command>
          <CommandInput placeholder="Search roles..." data-testid="role-selector-search" />
          <CommandList>
            <CommandEmpty>No roles found.</CommandEmpty>
            <CommandGroup>
              {roles.map((role) => (
                <CommandItem
                  key={role.id}
                  value={role.id}
                  onSelect={() => handleToggleRole(role.id)}
                  data-testid={`role-selector-item-${role.id}`}
                >
                  <Check
                    className={cn(
                      'mr-2 h-4 w-4',
                      selectedRoleIds.includes(role.id) ? 'opacity-100' : 'opacity-0'
                    )}
                  />
                  {role.name}
                </CommandItem>
              ))}
            </CommandGroup>
          </CommandList>
        </Command>
      </PopoverContent>
    </Popover>
  )
}
