import './styles.css'
import { StrictMode } from 'react'
import { createRouter } from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from '@tanstack/react-router'
import ReactDOM from 'react-dom/client'

// Import route tree
import { routeTree } from './routeTree.gen'

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,
      retry: false,
    },
  },
})

// Create router with context
const router = createRouter({
  routeTree,
  context: {
    queryClient,
  },
})

// Register router for TypeScript types
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
  interface RouterContext {
    queryClient: QueryClient
  }
}

// Expose router and QueryClient to window for debugging
if (import.meta.env.DEV) {
  window.router = router
  console.log('[Router Debug] Router exposed to window.router')
}

window.__REACT_QUERY_CLIENT__ = queryClient

// Render app
const rootElement = document.getElementById('app')!

ReactDOM.createRoot(rootElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>
)
