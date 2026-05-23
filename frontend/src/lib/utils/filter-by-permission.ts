interface FilterableItem {
  permission?: string | null
  id?: string
  children?: FilterableItem[]
}

export function filterByPermission<T extends FilterableItem>(
  items: T[],
  permissions: string[],
  realmId?: string
): T[] {
  return items
    .filter((item) => {
      if (item.id === 'realms' && realmId !== undefined && realmId !== 'admin') return false
      if (item.permission && !permissions.includes(item.permission)) return false
      return true
    })
    .map((item) => {
      if (item.children && item.children.length > 0) {
        const filteredChildren = filterByPermission(item.children, permissions, realmId)
        return { ...item, children: filteredChildren } as T
      }
      return item
    })
    .filter((item) => {
      if (item.children && item.children.length === 0) return false
      return true
    })
}
