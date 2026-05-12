import { useState, useCallback } from 'react'

export function useDialogState<T extends { id?: string | number } = Record<string, never>>() {
  const [open, setOpen] = useState(false)
  const [data, setData] = useState<Partial<T>>({})

  const openWith = useCallback((item: Partial<T>) => {
    setData(item)
    setOpen(true)
  }, [])

  return {
    open,
    data,
    setOpen,
    openWith,
    isEditing: data.id !== undefined,
  }
}
