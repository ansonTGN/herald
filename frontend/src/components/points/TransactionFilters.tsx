import { z } from 'zod'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Calendar, X } from 'lucide-react'
import { optionalStringEnum } from '@/lib/form-utils'
import { toDateInputValue, toUtcDateRangeBoundary } from '@/lib/date-utils'
import { FILTER_ALL_VALUE } from '@/lib/constants'
import type { TransactionFilters as TransactionFiltersType } from '@/lib/schemas/points-forms'

const transactionTypeEnum = z.enum(['recharge', 'consume'])

interface TransactionFiltersProps {
  filters: TransactionFiltersType
  onChange: (filters: TransactionFiltersType) => void
  onClear: () => void
  clientApps?: Array<{ id: string; name: string }>
  admin?: boolean
  loading?: boolean
  className?: string
}

export function TransactionFilters({
  filters,
  onChange,
  onClear,
  clientApps,
  admin = false,
  loading,
  className,
}: TransactionFiltersProps) {
  const form = useAppForm({
    schema: z.object({
      transactionType: optionalStringEnum(transactionTypeEnum),
      startTime: z.string().optional(),
      endTime: z.string().optional(),
      clientAppId: z.string().optional(),
    }),
    defaultValues: {
      transactionType: filters.transactionType,
      startTime: toDateInputValue(filters.startTime),
      endTime: toDateInputValue(filters.endTime),
      clientAppId: filters.clientAppId || '',
    },
    onSubmit: async ({ value }) => {
      const newFilters: TransactionFiltersType = {
        transactionType: value.transactionType,
        startTime: value.startTime ? toUtcDateRangeBoundary(value.startTime, 'start') : undefined,
        endTime: value.endTime ? toUtcDateRangeBoundary(value.endTime, 'end') : undefined,
        clientAppId: value.clientAppId || undefined,
      }
      onChange(newFilters)
    },
  })

  function handleReset() {
    form.reset()
    onClear()
  }

  function hasActiveFilters() {
    return !!(
      filters.transactionType ||
      filters.startTime ||
      filters.endTime ||
      filters.clientAppId
    )
  }

  return (
    <div className={className} data-testid="transaction-filters">
      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
      >
        <AppForm>
          <div className="space-y-4">
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
              {/* Transaction Type */}
              <div className="space-y-2">
                <form.Field name="transactionType">
                  {(field) => (
                    <>
                      <Label htmlFor={field.name}>Transaction Type</Label>
                      <Select
                        value={field.state.value ?? FILTER_ALL_VALUE}
                        onValueChange={(value) =>
                          field.handleChange(
                            value === FILTER_ALL_VALUE
                              ? undefined
                              : (value as 'recharge' | 'consume')
                          )
                        }
                      >
                        <SelectTrigger id={field.name} data-testid="filter-transaction-type">
                          <SelectValue placeholder="All types" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value={FILTER_ALL_VALUE}>All types</SelectItem>
                          <SelectItem value="recharge">Recharge</SelectItem>
                          <SelectItem value="consume">Consume</SelectItem>
                        </SelectContent>
                      </Select>
                    </>
                  )}
                </form.Field>
              </div>

              {/* From Date */}
              <div className="space-y-2">
                <form.Field name="startTime">
                  {(field) => (
                    <>
                      <Label htmlFor={field.name}>From Date</Label>
                      <div className="relative">
                        <Input
                          id={field.name}
                          type="date"
                          value={field.state.value ?? ''}
                          onChange={(e) => field.handleChange(e.target.value)}
                          data-testid="filter-from-date"
                        />
                        <Calendar className="absolute right-3 top-2.5 h-4 w-4 text-muted-foreground" />
                      </div>
                    </>
                  )}
                </form.Field>
              </div>

              {/* To Date */}
              <div className="space-y-2">
                <form.Field name="endTime">
                  {(field) => (
                    <>
                      <Label htmlFor={field.name}>To Date</Label>
                      <div className="relative">
                        <Input
                          id={field.name}
                          type="date"
                          value={field.state.value ?? ''}
                          onChange={(e) => field.handleChange(e.target.value)}
                          data-testid="filter-to-date"
                        />
                        <Calendar className="absolute right-3 top-2.5 h-4 w-4 text-muted-foreground" />
                      </div>
                    </>
                  )}
                </form.Field>
              </div>

              {/* Client App (admin only) */}
              {admin && (
                <div className="space-y-2">
                  <form.Field name="clientAppId">
                    {(field) => (
                      <>
                        <Label htmlFor={field.name}>Client App</Label>
                        <Select
                          value={field.state.value ?? FILTER_ALL_VALUE}
                          onValueChange={(value) =>
                            field.handleChange(value === FILTER_ALL_VALUE ? '' : value)
                          }
                        >
                          <SelectTrigger id={field.name} data-testid="filter-client-app">
                            <SelectValue placeholder="All apps" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value={FILTER_ALL_VALUE}>All apps</SelectItem>
                            {clientApps?.map((app) => (
                              <SelectItem key={app.id} value={app.id}>
                                {app.name}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      </>
                    )}
                  </form.Field>
                </div>
              )}
            </div>

            {/* Action Buttons */}
            <div className="flex justify-end gap-2">
              {hasActiveFilters() && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={handleReset}
                  disabled={loading}
                  data-testid="clear-filters-button"
                >
                  <X className="mr-2 h-4 w-4" />
                  Clear Filters
                </Button>
              )}
              <Button type="submit" size="sm" disabled={loading} data-testid="apply-filters-button">
                {loading ? 'Applying...' : 'Apply Filters'}
              </Button>
            </div>
          </div>
        </AppForm>
      </form>
    </div>
  )
}
