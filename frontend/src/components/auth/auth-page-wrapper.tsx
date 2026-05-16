interface AuthPageWrapperProps {
  children: React.ReactNode
}

export function AuthPageWrapper({ children }: AuthPageWrapperProps) {
  return <div className="flex min-h-screen items-center justify-center bg-gradient-to-b from-background to-muted/30">{children}</div>
}
