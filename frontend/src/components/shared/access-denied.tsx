interface AccessDeniedProps {
  message?: string
}

export function AccessDenied({
  message = 'Access denied: You do not have permission to view this page',
}: AccessDeniedProps) {
  return <div className="text-destructive">{message}</div>
}
