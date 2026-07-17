import './styles.css'
import { StrictMode } from 'react'
import { createRouter } from '@tanstack/react-router'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from '@tanstack/react-router'
import ReactDOM from 'react-dom/client'
import { LocaleProvider } from '@/components/shared/locale-provider'

// Initialize the Bearer API client (request + 401-refresh interceptors) BEFORE
// any module that issues generated-client calls (route loaders, QueryClient).
// This wires `Authorization: Bearer` from the in-memory access-token holder
// onto every SDK request and silently refreshes once on a 401 (design §4.4).
import { initBearerClient } from '@/lib/api-client'
initBearerClient()

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
      <LocaleProvider>
        <RouterProvider router={router} />
      </LocaleProvider>
    </QueryClientProvider>
  </StrictMode>
)
