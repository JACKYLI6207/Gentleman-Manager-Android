import type { SnapshotCategoryHeader } from './api'
import { CATEGORY_SEARCH_SCOPES } from './categories'

export type CategorySearchScope = { label: string; cateId: number }

export function resolveCategoryScopeFromComicCategory(category: string): CategorySearchScope | null {
  const normalized = category.trim()
  if (normalized === '') {
    return null
  }
  const exact = CATEGORY_SEARCH_SCOPES.find((option) => option.label === normalized)
  if (exact !== undefined) {
    return exact
  }
  return (
    CATEGORY_SEARCH_SCOPES.find(
      (option) => normalized.includes(option.label) || option.label.includes(normalized),
    ) ?? null
  )
}

export function findLatestSnapshotHeaderForScope(
  headers: SnapshotCategoryHeader[],
  scope: CategorySearchScope,
): SnapshotCategoryHeader | undefined {
  const matching = headers.filter((h) => h.cateId === scope.cateId)
  if (matching.length === 0) {
    const byLabel = headers.filter(
      (h) => h.label === scope.label || h.label.includes(scope.label) || scope.label.includes(h.label),
    )
    if (byLabel.length === 0) return undefined
    return [...byLabel].sort((a, b) => (b.modifiedMs ?? 0) - (a.modifiedMs ?? 0))[0]
  }
  return [...matching].sort((a, b) => (b.modifiedMs ?? 0) - (a.modifiedMs ?? 0))[0]
}

export function hasComicCategorySnapshot(
  category: string,
  headers: SnapshotCategoryHeader[],
): boolean {
  const scope = resolveCategoryScopeFromComicCategory(category)
  if (scope === null || headers.length === 0) {
    return false
  }
  return findLatestSnapshotHeaderForScope(headers, scope) !== undefined
}
