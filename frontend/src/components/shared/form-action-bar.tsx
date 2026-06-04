import type { ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { Loader2 } from 'lucide-react'
import { m } from '@/paraglide/messages'

interface FormActionBarProps {
  onCancel: () => void
  isSubmitting: boolean
  isEditing: boolean
  cancelTestId: string
  submitTestId: string
  children?: ReactNode
}

export function FormActionBar({
  onCancel,
  isSubmitting,
  isEditing,
  cancelTestId,
  submitTestId,
  children,
}: FormActionBarProps) {
  return (
    <div className="flex items-center gap-3 pt-4 border-t">
      <Button type="button" variant="outline" onClick={onCancel} data-testid={cancelTestId}>
        {m['common.cancel']()}
      </Button>
      {children}
      <Button type="submit" disabled={isSubmitting} data-testid={submitTestId}>
        {isSubmitting ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            {m['shared.saving']()}
          </>
        ) : isEditing ? (
          m['shared.save_changes']()
        ) : (
          m['shared.create_configuration']()
        )}
      </Button>
    </div>
  )
}
