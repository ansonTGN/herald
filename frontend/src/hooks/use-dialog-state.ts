import { useState, useCallback } from 'react'

export function useDialogManager<T>() {
  const [isOpen, setIsOpen] = useState(false)
  const [selectedItem, setSelectedItem] = useState<T | null>(null)

  const open = useCallback((item?: T) => {
    setSelectedItem(item ?? null)
    setIsOpen(true)
  }, [])

  const close = useCallback(() => {
    setIsOpen(false)
    setSelectedItem(null)
  }, [])

  const onOpenChange = useCallback((v: boolean) => {
    if (!v) close()
  }, [close])

  return { isOpen, selectedItem, open, close, onOpenChange }
}

/** @deprecated Use useDialogManager instead */
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
