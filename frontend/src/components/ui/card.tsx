import * as React from 'react'

import { cn } from '@/lib/utils'

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  'data-testid'?: string
  ref?: React.RefObject<HTMLDivElement>
}

export function Card({ className, 'data-testid': dataTestId, ref, ...props }: CardProps) {
  return (
    <div
      ref={ref}
      className={cn('rounded-xl border bg-card text-card-foreground shadow', className)}
      data-testid={dataTestId}
      {...props}
    />
  )
}

export interface CardHeaderProps extends React.HTMLAttributes<HTMLDivElement> {
  'data-testid'?: string
  ref?: React.RefObject<HTMLDivElement>
}

export function CardHeader({
  className,
  'data-testid': dataTestId,
  ref,
  ...props
}: CardHeaderProps) {
  return (
    <div
      ref={ref}
      className={cn('flex flex-col space-y-1.5 p-6', className)}
      data-testid={dataTestId}
      {...props}
    />
  )
}

export interface CardTitleProps extends React.HTMLAttributes<HTMLHeadingElement> {
  'data-testid'?: string
  ref?: React.RefObject<HTMLHeadingElement>
}

export function CardTitle({ className, 'data-testid': dataTestId, ref, ...props }: CardTitleProps) {
  return (
    <h3
      ref={ref}
      className={cn('font-semibold leading-none tracking-tight', className)}
      data-testid={dataTestId}
      {...props}
    />
  )
}

export interface CardDescriptionProps extends React.HTMLAttributes<HTMLParagraphElement> {
  ref?: React.RefObject<HTMLParagraphElement>
}

export function CardDescription({ className, ref, ...props }: CardDescriptionProps) {
  return <p ref={ref} className={cn('text-sm text-muted-foreground', className)} {...props} />
}

export interface CardContentProps extends React.HTMLAttributes<HTMLDivElement> {
  'data-testid'?: string
  ref?: React.RefObject<HTMLDivElement>
}

export function CardContent({
  className,
  'data-testid': dataTestId,
  ref,
  ...props
}: CardContentProps) {
  return <div ref={ref} className={cn('p-6 pt-0', className)} data-testid={dataTestId} {...props} />
}

export interface CardFooterProps extends React.HTMLAttributes<HTMLDivElement> {
  ref?: React.RefObject<HTMLDivElement>
}

export function CardFooter({ className, ref, ...props }: CardFooterProps) {
  return <div ref={ref} className={cn('flex items-center p-6 pt-0', className)} {...props} />
}
