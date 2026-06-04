import { ReactQueryDevtools } from '@tanstack/react-query-devtools'
// import { TanStackRouterDevtools } from '@tanstack/router-devtools'

export function Devtools() {
  return (
    <>
      <ReactQueryDevtools buttonPosition="bottom-right" />
      {/* <div className="fixed bottom-0 left-0 z-50">
        <TanStackRouterDevtools panelProps={{ style: { overflow: 'hidden' } }} />
      </div> */}
    </>
  )
}
