import { useState, useMemo } from 'react'
import { Check, ChevronsUpDown } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
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

export interface MultiselectOption {
  id: string
  /** Searchable + display label. */
  label: string
  /** Secondary info shown after the label, e.g. clientId / provider. */
  hint?: string
}

interface BucketMultiselectProps {
  options: MultiselectOption[]
  value: string[]
  onChange: (next: string[]) => void
  placeholder?: string
  searchPlaceholder?: string
  emptyText?: string
  /** testid applied to the trigger and used as prefix for item testids. */
  testIdPrefix: string
  disabled?: boolean
  error?: string
}

/**
 * Shared Popover + Command multi-select (pattern reused from
 * `components/shared/client-app-selector.tsx`). Each option toggles membership
 * in the selected set; a Check icon reflects state. The trigger surfaces a
 * count Badge plus the joined labels of the current selection.
 *
 * Reused by the coverage-set and entitlement-mappings multiselects.
 */
export function BucketMultiselect({
  options,
  value,
  onChange,
  placeholder,
  searchPlaceholder,
  emptyText,
  testIdPrefix,
  disabled = false,
  error,
}: BucketMultiselectProps) {
  const [open, setOpen] = useState(false)

  const selectedLabels = useMemo(() => {
    const map = new Map(options.map((o) => [o.id, o]))
    return value.map((id) => map.get(id)?.label ?? id)
  }, [options, value])

  function toggle(id: string) {
    onChange(value.includes(id) ? value.filter((v) => v !== id) : [...value, id])
  }

  return (
    <div className="space-y-1.5">
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger asChild>
          <Button
            type="button"
            variant="outline"
            role="combobox"
            aria-expanded={open}
            disabled={disabled}
            className="w-full justify-between font-normal"
            data-testid={testIdPrefix}
          >
            <span
              className={cn(
                'flex items-center gap-1 truncate',
                value.length === 0 && 'text-muted-foreground'
              )}
            >
              {value.length === 0 ? (
                placeholder
              ) : (
                <>
                  <Badge variant="secondary" className="font-semibold">
                    {value.length}
                  </Badge>
                  <span className="truncate">{selectedLabels.join(', ')}</span>
                </>
              )}
            </span>
            <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
          </Button>
        </PopoverTrigger>
        <PopoverContent
          className="w-[var(--radix-popover-content-available-width)] min-w-[16rem] p-0"
          align="start"
        >
          <Command>
            <CommandInput placeholder={searchPlaceholder} data-testid={`${testIdPrefix}-search`} />
            <CommandList>
              <CommandEmpty>{emptyText}</CommandEmpty>
              <CommandGroup>
                {options.map((opt) => {
                  const selected = value.includes(opt.id)
                  return (
                    <CommandItem
                      key={opt.id}
                      value={`${opt.label} ${opt.hint ?? ''} ${opt.id}`}
                      onSelect={() => toggle(opt.id)}
                      data-testid={`${testIdPrefix}-item-${opt.id}`}
                    >
                      <Check
                        className={cn('mr-2 h-4 w-4', selected ? 'opacity-100' : 'opacity-0')}
                      />
                      <span className="truncate">{opt.label}</span>
                      {opt.hint && (
                        <span className="ml-auto text-xs text-muted-foreground">{opt.hint}</span>
                      )}
                    </CommandItem>
                  )
                })}
              </CommandGroup>
            </CommandList>
          </Command>
        </PopoverContent>
      </Popover>
      {error && (
        <p className="text-sm text-destructive" role="alert" data-testid={`${testIdPrefix}-error`}>
          {error}
        </p>
      )}
    </div>
  )
}

/**
 * Coverage-set multiselect: client apps covered by a bucket.
 * At-least-one enforcement lives in `createCreditBucketSchema` /
 * `updateCreditBucketSchema` (`clientAppIds.min(1)`); the editor passes the
 * schema-derived error down via `error`.
 */
export function CreditBucketCoverageMultiselect(
  props: Omit<BucketMultiselectProps, 'testIdPrefix'> & { testIdPrefix?: string }
) {
  return (
    <BucketMultiselect
      {...props}
      placeholder={props.placeholder || m['credit_buckets.coverage_placeholder']()}
      searchPlaceholder={
        props.searchPlaceholder || m['credit_buckets.coverage_search_placeholder']()
      }
      emptyText={props.emptyText || m['credit_buckets.coverage_empty']()}
      testIdPrefix={props.testIdPrefix ?? 'bucket-coverage-multiselect'}
    />
  )
}
