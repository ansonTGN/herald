import { m } from '@/paraglide/messages'

interface AccessDeniedProps {
  message?: string
}

export function AccessDenied({ message }: AccessDeniedProps) {
  return <div className="text-destructive">{message ?? m['error.access_denied']()}</div>
}
