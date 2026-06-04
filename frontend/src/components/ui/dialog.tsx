import * as React from 'react'
import { XIcon } from 'lucide-react'
import {
  Root as DialogPrimitive,
  Trigger as DialogTriggerPrimitive,
  Portal as DialogPortalPrimitive,
  Close as DialogClosePrimitive,
  Overlay as DialogOverlayPrimitive,
  Content as DialogContentPrimitive,
  Title as DialogTitlePrimitive,
  Description as DialogDescriptionPrimitive,
} from '@radix-ui/react-dialog'

import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

function Dialog({ ...props }: React.ComponentProps<typeof DialogPrimitive>) {
  return <DialogPrimitive data-slot="dialog" {...props} />
}

function DialogTrigger({ ...props }: React.ComponentProps<typeof DialogTriggerPrimitive>) {
  return <DialogTriggerPrimitive data-slot="dialog-trigger" {...props} />
}

function DialogPortal({ ...props }: React.ComponentProps<typeof DialogPortalPrimitive>) {
  return <DialogPortalPrimitive data-slot="dialog-portal" {...props} />
}

function DialogClose({ ...props }: React.ComponentProps<typeof DialogClosePrimitive>) {
  return <DialogClosePrimitive data-slot="dialog-close" {...props} />
}

function DialogOverlay({
  className,
  ...props
}: React.ComponentProps<typeof DialogOverlayPrimitive>) {
  return (
    <DialogOverlayPrimitive
      data-slot="dialog-overlay"
      className={cn(
        'data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 fixed inset-0 z-50 bg-black/50',
        className
      )}
      {...props}
    />
  )
}

function DialogContent({
  className,
  children,
  showCloseButton = true,
  ...props
}: React.ComponentProps<typeof DialogContentPrimitive> & {
  showCloseButton?: boolean
}) {
  return (
    <DialogPortal data-slot="dialog-portal">
      <DialogOverlay />
      <DialogContentPrimitive
        data-testid="dialog"
        data-slot="dialog-content"
        className={cn(
          'bg-background data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 fixed top-[50%] left-[50%] z-50 grid w-full max-w-[calc(100%-2rem)] max-h-[calc(100vh-4rem)] translate-x-[-50%] translate-y-[-50%] gap-4 rounded-lg border p-6 shadow-lg duration-200 outline-none overflow-y-auto sm:max-w-lg',
          className
        )}
        {...props}
      >
        {children}
        {showCloseButton && (
          <DialogClosePrimitive
            data-slot="dialog-close"
            className="ring-offset-background focus:ring-ring data-[state=open]:bg-accent data-[state=open]:text-muted-foreground absolute top-4 right-4 rounded-xs opacity-70 transition-opacity hover:opacity-100 focus:ring-2 focus:ring-offset-2 focus:outline-hidden disabled:pointer-events-none [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"
          >
            <XIcon />
            <span className="sr-only">Close</span>
          </DialogClosePrimitive>
        )}
      </DialogContentPrimitive>
    </DialogPortal>
  )
}

function DialogHeader({ className, ...props }: React.ComponentProps<'div'>) {
  return (
    <div
      data-slot="dialog-header"
      className={cn('flex flex-col gap-2 text-center sm:text-left', className)}
      {...props}
    />
  )
}

function DialogFooter({
  className,
  showCloseButton = false,
  children,
  ...props
}: React.ComponentProps<'div'> & {
  showCloseButton?: boolean
}) {
  return (
    <div
      data-slot="dialog-footer"
      className={cn('flex flex-col-reverse gap-2 sm:flex-row sm:justify-end', className)}
      {...props}
    >
      {children}
      {showCloseButton && (
        <DialogClosePrimitive asChild>
          <Button variant="outline">Close</Button>
        </DialogClosePrimitive>
      )}
    </div>
  )
}

function DialogTitle({ className, ...props }: React.ComponentProps<typeof DialogTitlePrimitive>) {
  return (
    <DialogTitlePrimitive
      data-slot="dialog-title"
      data-testid="dialog-title"
      className={cn('text-lg leading-none font-semibold', className)}
      {...props}
    />
  )
}

function DialogDescription({
  className,
  ...props
}: React.ComponentProps<typeof DialogDescriptionPrimitive>) {
  return (
    <DialogDescriptionPrimitive
      data-slot="dialog-description"
      className={cn('text-muted-foreground text-sm', className)}
      {...props}
    />
  )
}

// ==================== Alert Dialog ====================

const AlertDialog = Dialog

const AlertDialogTrigger = DialogTrigger

const AlertDialogContent = React.forwardRef<
  React.ElementRef<typeof DialogContentPrimitive>,
  React.ComponentProps<typeof DialogContentPrimitive>
>(({ className, ...props }, ref) => (
  <DialogContent ref={ref} className={cn('gap-4', className)} {...props} showCloseButton={false} />
))
AlertDialogContent.displayName = 'AlertDialogContent'

const AlertDialogHeader = ({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) => (
  <div className={cn('flex flex-col gap-2 text-center sm:text-left', className)} {...props} />
)
AlertDialogHeader.displayName = 'AlertDialogHeader'

const AlertDialogFooter = ({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn('flex flex-col-reverse gap-2 sm:flex-row sm:justify-end', className)}
    {...props}
  />
)
AlertDialogFooter.displayName = 'AlertDialogFooter'

const AlertDialogTitle = ({
  className,
  ...props
}: React.ComponentProps<typeof DialogTitlePrimitive>) => (
  <DialogTitle className={cn('text-lg font-semibold', className)} {...props} />
)
AlertDialogTitle.displayName = 'AlertDialogTitle'

const AlertDialogDescription = ({
  className,
  ...props
}: React.ComponentProps<typeof DialogDescriptionPrimitive>) => (
  <DialogDescription className={cn('text-sm text-muted-foreground', className)} {...props} />
)
AlertDialogDescription.displayName = 'AlertDialogDescription'

const AlertDialogAction = React.forwardRef<
  React.ElementRef<typeof Button>,
  React.ComponentProps<typeof Button>
>(({ className, ...props }, ref) => <Button ref={ref} className={className} {...props} />)
AlertDialogAction.displayName = 'AlertDialogAction'

const AlertDialogCancel = React.forwardRef<
  React.ElementRef<typeof Button>,
  React.ComponentProps<typeof Button>
>(({ className, ...props }, ref) => (
  <Button ref={ref} variant="outline" className={cn('mt-2 sm:mt-0', className)} {...props} />
))
AlertDialogCancel.displayName = 'AlertDialogCancel'

export {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
  DialogTrigger,
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
}
