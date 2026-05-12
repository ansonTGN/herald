import { useState } from 'react'
import { ChevronDown, Settings } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible'
import { Button } from '@/components/ui/button'

interface AdvancedSettingsCollapsibleProps {
  children: React.ReactNode
  dataTestId?: string
}

/**
 * AdvancedSettingsCollapsible - Collapsible panel for advanced security options
 *
 * Features:
 * - Collapsed by default to reduce cognitive load
 * - Smooth expand/collapse animations
 * - Visual indicator showing number of options
 * - Accessible keyboard navigation
 * - Consistent with project's Collapsible component pattern
 */
export function AdvancedSettingsCollapsible({
  children,
  dataTestId = 'advanced-settings-collapsible',
}: AdvancedSettingsCollapsibleProps) {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <Collapsible open={isOpen} onOpenChange={setIsOpen}>
      <div className="mt-6" data-testid={dataTestId}>
        <CollapsibleTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            className={cn(
              'w-full justify-between px-4 py-3 h-auto border-2 transition-all',
              'hover:bg-accent active:scale-[0.99]',
              isOpen ? 'border-primary bg-primary/5' : 'border-border border-dashed'
            )}
            data-testid={`${dataTestId}-trigger`}
          >
            <div className="flex items-center gap-3">
              <div
                className={cn(
                  'flex items-center justify-center w-10 h-10 rounded-lg transition-colors',
                  isOpen ? 'bg-primary/10' : 'bg-muted'
                )}
              >
                <Settings
                  className={cn(
                    'w-5 h-5 transition-colors',
                    isOpen ? 'text-primary' : 'text-muted-foreground'
                  )}
                />
              </div>
              <div className="flex-1 text-left">
                <div className="flex items-center gap-2">
                  <span className="font-semibold text-foreground">Advanced Security Settings</span>
                  <span className="px-2 py-0.5 text-xs font-medium bg-muted text-muted-foreground rounded">
                    8 options
                  </span>
                </div>
                <p className="text-sm text-muted-foreground mt-0.5">
                  {isOpen
                    ? 'Configure fine-grained security policies'
                    : 'Additional security options for advanced use cases'}
                </p>
              </div>
            </div>
            <ChevronDown
              className={cn(
                'w-5 h-5 text-muted-foreground transition-transform duration-200',
                isOpen && 'rotate-180'
              )}
            />
          </Button>
        </CollapsibleTrigger>

        <CollapsibleContent className="pt-3">
          <div
            className="rounded-lg border border-border bg-muted/50 p-6 space-y-4"
            data-testid={`${dataTestId}-content`}
          >
            {children}
          </div>
        </CollapsibleContent>
      </div>
    </Collapsible>
  )
}
