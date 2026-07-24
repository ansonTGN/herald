/**
 * Lazily injects a `<script src>` tag into `<head>` and tracks its load state.
 *
 * Idempotent across mounts: a module-level Map remembers each URL's status so
 * repeated mounts (React strict mode, re-renders) never inject a duplicate tag
 * or restart a load that is already in flight. Once a URL resolves to 'ready'
 * or 'error' it stays there for the lifetime of the page — browsers cache the
 * resource and a re-injection would not change the outcome.
 *
 * SSR-safe: returns 'idle' when `document` is undefined so it can be imported
 * in non-browser environments without throwing.
 *
 * Implemented with `useSyncExternalStore`: the script registry is an external
 * mutable source, and `useSyncExternalStore` is the React-blessed way to read
 * it without the cascading-render smell of `setState` inside an effect.
 */
import { useSyncExternalStore } from 'react'

export type ScriptStatus = 'idle' | 'loading' | 'ready' | 'error'

type ScriptEntry = {
  status: ScriptStatus
  version: number // bumped on each status change so caches invalidate
  // Listeners notified on every status transition for this URL.
  listeners: Set<() => void>
}

// Per-URL registry shared across all callers in the page. Module-level because
// script load state is a property of the document, not of any one component.
const scriptRegistry = new Map<string, ScriptEntry>()

function emit() {
  for (const entry of scriptRegistry.values()) {
    for (const listener of entry.listeners) listener()
  }
}

function readExistingStatus(src: string): ScriptStatus | undefined {
  if (typeof document === 'undefined') return undefined
  const existing = document.querySelector(`script[src="${src}"]`)
  if (!existing) return undefined
  // A tag added by us or by a prior load already completed successfully.
  return (existing.getAttribute('data-status') as ScriptStatus) ?? 'ready'
}

function loadScript(src: string): ScriptEntry {
  const existing = scriptRegistry.get(src)
  if (existing) return existing

  const cached = readExistingStatus(src)
  if (cached === 'ready' || cached === 'error') {
    const entry: ScriptEntry = { status: cached, version: 0, listeners: new Set() }
    scriptRegistry.set(src, entry)
    return entry
  }

  const entry: ScriptEntry = { status: 'loading', version: 0, listeners: new Set() }
  scriptRegistry.set(src, entry)

  const settle = (status: ScriptStatus) => {
    const current = scriptRegistry.get(src)
    if (!current || current.status === status) return
    current.status = status
    current.version++
    emit()
  }

  const script = document.createElement('script')
  script.src = src
  script.async = true
  script.defer = true
  script.setAttribute('data-status', 'loading')
  script.addEventListener('load', () => {
    script.setAttribute('data-status', 'ready')
    settle('ready')
  })
  script.addEventListener('error', () => {
    script.setAttribute('data-status', 'error')
    settle('error')
  })
  document.head.appendChild(script)

  return entry
}

/**
 * Inject `src` and subscribe to its load status. Returns 'idle' when `src` is
 * null or in non-browser environments; otherwise the current status, updating
 * as the script settles.
 */
export function useScript(src: string | null | undefined): ScriptStatus {
  // Key the snapshot by (src, per-entry version, global version) so React
  // detects changes across renders. getSnapshot must return a stable primitive
  // for the same underlying state or useSyncExternalStore loops.
  const subscribe = (onStoreChange: () => void) => {
    if (!src || typeof document === 'undefined') return () => {}
    const entry = loadScript(src)
    entry.listeners.add(onStoreChange)
    return () => {
      entry.listeners.delete(onStoreChange)
    }
  }
  const getSnapshot = (): string => {
    if (!src || typeof document === 'undefined') return 'idle|null'
    const entry = scriptRegistry.get(src)
    const status = entry?.status ?? readExistingStatus(src) ?? 'idle'
    return `${src}|${status}|${entry?.version ?? 0}`
  }
  const getServerSnapshot = () => 'idle|server'

  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot)
  // Parse the status back out of the snapshot key (src is the hook input, so
  // only the status portion varies here).
  if (!src || typeof document === 'undefined') return 'idle'
  const status = snapshot.split('|')[1] as ScriptStatus
  return status === 'idle' ? 'idle' : status
}
