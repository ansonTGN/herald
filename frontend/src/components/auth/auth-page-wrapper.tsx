interface AuthPageWrapperProps {
  children: React.ReactNode
}

export function AuthPageWrapper({ children }: AuthPageWrapperProps) {
  return <div className="flex min-h-screen items-center justify-center bg-gray-50">{children}</div>
}
