import * as React from 'react'

import { cn } from '@/lib/utils'

export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'ref'> {
  'data-testid'?: string
  ref?: React.Ref<HTMLInputElement>
}

export function Input({ className, type, 'data-testid': dataTestId, ref, ...props }: InputProps) {
  return (
    <input
      type={type}
      className={cn(
        'flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-base shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-placeholder focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:border-transparent disabled:bg-muted disabled:text-muted-foreground md:text-sm',
        className
      )}
      ref={ref}
      data-testid={dataTestId}
      {...props}
    />
  )
}

Input.displayName = 'Input'
