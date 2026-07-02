import { useState } from 'react'
import { Plus, Trash2, Info } from 'lucide-react'
import type { QuotaWindowInputDto } from '@/lib/api-generated/types.gen'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { Tooltip, TooltipContent, TooltipTrigger, TooltipProvider } from '@/components/ui/tooltip'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { cn } from '@/lib/utils'
import { m } from '@/paraglide/messages'

export type QuotaEditorContext = 'entitlement-mapping' | 'realm-default'

export interface QuotaWindowFieldError {
  windowSeconds?: string
  limit?: string
}

export interface MultiWindowQuotaEditorProps {
  /** Controlled array of quota windows. */
  value: QuotaWindowInputDto[]
  /** Emits the next full array on any edit / add / remove. */
  onChange: (value: QuotaWindowInputDto[]) => void
  /** Disables all controls (inputs, select, add, delete). */
  disabled?: boolean
  /** Drives only the impact-alert wording; no behavioral difference. */
  context: QuotaEditorContext
  /** Base for `data-testid` attributes. Defaults to `quota-window`. */
  testIdPrefix?: string
  /**
   * Optional per-row validation errors keyed by row index. Each entry may
   * carry a `windowSeconds` and/or `limit` message; when present the
   * corresponding input renders in an error state with an inline message.
   * The editor itself stays page-agnostic — pages own cross-row validation
   * (e.g. uniqueness) and the save gate.
   */
  error?: Record<number, QuotaWindowFieldError>
}

/** Design §6: hard cap on the number of windows. */
const MAX_WINDOWS = 8

/**
 * Supported unit selector options. The last option (months-30d) is fixed at
 * 30 days to keep the editor deterministic; `month` is intentionally avoided
 * as a calendar unit since `windowSeconds` cannot represent variable-length
 * months.
 */
const WINDOW_UNITS = [
  { value: 'seconds', factor: 1, label: 'seconds' },
  { value: 'minutes', factor: 60, label: 'minutes' },
  { value: 'hours', factor: 60 * 60, label: 'hours' },
  { value: 'days', factor: 60 * 60 * 24, label: 'days' },
  { value: 'weeks', factor: 60 * 60 * 24 * 7, label: 'weeks' },
  { value: 'months-30d', factor: 60 * 60 * 24 * 30, label: 'months (30d)' },
] as const

type WindowUnitValue = (typeof WINDOW_UNITS)[number]['value']

/**
 * Derives the largest unit that divides `windowSeconds` evenly, so the length
 * input shows a readable number (e.g. 5 hours instead of 18000 seconds).
 * Falls back to seconds when no larger unit divides evenly.
 */
function deriveUnit(windowSeconds: number): { unit: WindowUnitValue; amount: number } {
  for (let i = WINDOW_UNITS.length - 1; i >= 0; i -= 1) {
    const { factor, value } = WINDOW_UNITS[i]
    if (windowSeconds > 0 && windowSeconds % factor === 0) {
      return { unit: value, amount: windowSeconds / factor }
    }
  }
  return { unit: 'seconds', amount: windowSeconds }
}

function unitFactor(unit: WindowUnitValue): number {
  return WINDOW_UNITS.find((u) => u.value === unit)?.factor ?? 1
}

/**
 * MultiWindowQuotaEditor — controlled, page-agnostic row editor for
 * multi-window quota definitions.
 *
 * Embeddable in the entitlement-mapping page (`context="entitlement-mapping"`)
 * and the realm-default free-period page (`context="realm-default"`). The only
 * `context`-driven difference is the impact-alert wording. Behavior is driven
 * purely by props. Accepts and emits `QuotaWindowInputDto[]`; per-window
 * validation rules (`windowSeconds > 0`, `limit >= 0`, max 8 windows) are
 * declared in `quotaWindowSchema` / `quotaWindowsSchema` in
 * `@/lib/schemas/points-forms`; pages compose those schemas for their save
 * gate.
 */
export function MultiWindowQuotaEditor({
  value,
  onChange,
  disabled = false,
  context,
  testIdPrefix = 'quota-window',
  error,
}: MultiWindowQuotaEditorProps) {
  const atWindowCap = value.length >= MAX_WINDOWS
  const addDisabled = disabled || atWindowCap

  // Per-row display-unit overrides. `deriveUnit` picks a default unit from the
  // stored `windowSeconds`, but operators may switch the unit to enter/inspect
  // a length in friendlier terms (e.g. view 86400s as 24h). The override is
  // purely presentational — `windowSeconds` is the only value on the wire — so
  // it lives in local state and never flows through `onChange`.
  const [unitOverrides, setUnitOverrides] = useState<Record<number, WindowUnitValue>>({})

  const updateRow = (index: number, next: Partial<QuotaWindowInputDto>) => {
    onChange(value.map((window, i) => (i === index ? { ...window, ...next } : window)))
  }

  const removeRow = (index: number) => {
    onChange(value.filter((_, i) => i !== index))
  }

  const addRow = () => {
    if (addDisabled) return
    // Design step 4: default new window is 1 hour, limit 0.
    onChange([...value, { windowSeconds: 3600, limit: 0 }])
  }

  const impactText =
    context === 'entitlement-mapping'
      ? m['points.quota_editor_impact_mapping']()
      : m['points.quota_editor_impact_realm_default']()

  return (
    <div className="space-y-4" data-testid={`${testIdPrefix}-editor`}>
      <div className="flex items-center justify-end gap-1">
        <TooltipProvider delayDuration={200}>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                aria-label={m['points.quota_editor_impact_label']()}
                data-testid={`${testIdPrefix}-impact-tooltip`}
                className="text-muted-foreground hover:text-foreground"
              >
                <Info className="h-3.5 w-3.5" />
              </button>
            </TooltipTrigger>
            <TooltipContent data-testid={`${testIdPrefix}-impact-tooltip-content`}>
              {impactText}
            </TooltipContent>
          </Tooltip>
        </TooltipProvider>
      </div>

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{m['points.quota_editor_window_length']()}</TableHead>
            <TableHead>{m['points.quota_editor_window_unit']()}</TableHead>
            <TableHead>{m['points.quota_editor_window_limit']()}</TableHead>
            <TableHead className="w-10" />
          </TableRow>
        </TableHeader>
        <TableBody>
          {value.length === 0 && (
            <TableRow data-testid={`${testIdPrefix}-empty-row`}>
              <TableCell colSpan={4} className="text-muted-foreground text-sm">
                {m['common.no_data']()}
              </TableCell>
            </TableRow>
          )}
          {value.map((window, index) => {
            const derived = deriveUnit(window.windowSeconds)
            const unit = unitOverrides[index] ?? derived.unit
            const amount = window.windowSeconds / unitFactor(unit)
            const rowError = error?.[index]
            const lengthError = rowError?.windowSeconds
            const limitError = rowError?.limit
            return (
              <TableRow key={index} data-testid={`${testIdPrefix}-row-${index}`}>
                <TableCell>
                  <Input
                    type="number"
                    min={1}
                    step={1}
                    inputMode="numeric"
                    value={Number.isFinite(amount) ? amount : 0}
                    disabled={disabled}
                    aria-invalid={lengthError !== undefined}
                    aria-label={m['points.quota_editor_window_length']()}
                    data-testid={`${testIdPrefix}-length-row-${index}`}
                    className={cn('w-32', lengthError && 'border-destructive')}
                    onChange={(e) => {
                      const parsed = Number.parseInt(e.target.value, 10)
                      const safeAmount = Number.isFinite(parsed) ? parsed : 0
                      updateRow(index, {
                        windowSeconds: Math.max(0, safeAmount) * unitFactor(unit),
                      })
                    }}
                  />
                  {lengthError && (
                    <p className="text-destructive text-xs mt-1" role="alert">
                      {lengthError}
                    </p>
                  )}
                </TableCell>
                <TableCell>
                  <Select
                    value={unit}
                    disabled={disabled}
                    onValueChange={(next) => {
                      // Switching the unit reinterprets the displayed length
                      // number in the new unit (so "5" stays "5"), recomputing
                      // the absolute windowSeconds accordingly. This keeps the
                      // number stable and predictable while the unit selector
                      // acts as a true unit-conversion control.
                      const nextUnit = next as WindowUnitValue
                      const currentAmount = window.windowSeconds / unitFactor(unit)
                      const nextSeconds = Math.max(
                        0,
                        Math.round(currentAmount) * unitFactor(nextUnit)
                      )
                      updateRow(index, { windowSeconds: nextSeconds })
                      setUnitOverrides((prev) => ({
                        ...prev,
                        [index]: nextUnit,
                      }))
                    }}
                  >
                    <SelectTrigger
                      className="w-40"
                      aria-label={m['points.quota_editor_window_unit']()}
                      data-testid={`${testIdPrefix}-unit-row-${index}`}
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {WINDOW_UNITS.map((u) => (
                        <SelectItem key={u.value} value={u.value}>
                          {u.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </TableCell>
                <TableCell>
                  <Input
                    type="number"
                    min={0}
                    step={1}
                    inputMode="numeric"
                    value={Number.isFinite(window.limit) ? window.limit : 0}
                    disabled={disabled}
                    aria-invalid={limitError !== undefined}
                    aria-label={m['points.quota_editor_window_limit']()}
                    data-testid={`${testIdPrefix}-limit-row-${index}`}
                    className={cn('w-32', limitError && 'border-destructive')}
                    onChange={(e) => {
                      const parsed = Number.parseInt(e.target.value, 10)
                      updateRow(index, { limit: Number.isFinite(parsed) ? Math.max(0, parsed) : 0 })
                    }}
                  />
                  {limitError && (
                    <p className="text-destructive text-xs mt-1" role="alert">
                      {limitError}
                    </p>
                  )}
                </TableCell>
                <TableCell>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    disabled={disabled}
                    onClick={() => removeRow(index)}
                    aria-label={m['common.delete']()}
                    data-testid={`${testIdPrefix}-delete-row-${index}`}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </TableCell>
              </TableRow>
            )
          })}
        </TableBody>
      </Table>

      <div className="flex items-center gap-3">
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={addDisabled}
          onClick={addRow}
          data-testid={`${testIdPrefix}-add-button`}
        >
          <Plus className="h-4 w-4" />
          {m['points.quota_editor_add_window']()}
        </Button>
        {atWindowCap && (
          <Badge variant="secondary" data-testid={`${testIdPrefix}-window-cap`}>
            {m['points.quota_window_max']()}
          </Badge>
        )}
      </div>
    </div>
  )
}
