/** 官網頂部分類（對應 PC 版 categories.ts） */
export type SiteCategoryItem = {
  label: string
  cateId?: number
  browse?: 'home' | 'albums' | 'ranking'
  rankingCateId?: number | null
  children?: SiteCategoryItem[]
}

export function listRankingScopes(): { label: string; cateId?: number }[] {
  return [
    { label: '全部分類' },
    { label: '同人誌 / 日語', cateId: 12 },
    { label: '同人誌 / 漢化', cateId: 1 },
    { label: '單行本 / 漢化', cateId: 9 },
    { label: '韓漫 / 漢化', cateId: 20 },
  ]
}

export const SITE_CATEGORIES: SiteCategoryItem[] = [
  { label: '首頁', browse: 'home' },
  { label: '更新', browse: 'albums' },
  {
    label: '同人誌',
    cateId: 5,
    children: [
      { label: '漢化', cateId: 1 },
      { label: '日語', cateId: 12 },
      { label: 'English', cateId: 16 },
      { label: 'CG畫集', cateId: 2 },
      { label: 'AI圖集', cateId: 37 },
      { label: '3D漫畫', cateId: 22 },
      { label: 'Cosplay', cateId: 3 },
    ],
  },
  {
    label: '單行本',
    cateId: 6,
    children: [
      { label: '漢化', cateId: 9 },
      { label: '日語', cateId: 13 },
      { label: 'English', cateId: 17 },
    ],
  },
  {
    label: '雜誌&短篇',
    cateId: 7,
    children: [
      { label: '漢化', cateId: 10 },
      { label: '日語', cateId: 14 },
      { label: 'English', cateId: 18 },
    ],
  },
  {
    label: '韓漫',
    cateId: 19,
    children: [
      { label: '漢化', cateId: 20 },
      { label: '其他', cateId: 21 },
    ],
  },
  {
    label: '排行',
    browse: 'ranking',
    children: listRankingScopes().map((scope) => ({
      label: scope.label,
      rankingCateId: scope.cateId ?? null,
    })),
  },
]

export function listCategorySearchScopes(): { label: string; cateId: number }[] {
  const out: { label: string; cateId: number }[] = []

  function walk(items: SiteCategoryItem[], parentLabel?: string) {
    for (const item of items) {
      if (item.browse !== undefined) {
        continue
      }
      const label = parentLabel !== undefined ? `${parentLabel} / ${item.label}` : item.label
      if (item.cateId !== undefined) {
        out.push({ label, cateId: item.cateId })
      }
      if (item.children !== undefined) {
        walk(item.children, item.label)
      }
    }
  }

  walk(SITE_CATEGORIES)
  return out
}

export const CATEGORY_SEARCH_SCOPES = listCategorySearchScopes()

/** 舊快照 meta 缺 scanTargetCateId 時，依標籤對應官網分類 ID */
export function resolveCateIdFromLabel(label: string): number | undefined {
  const trimmed = label.trim()
  if (!trimmed) return undefined
  const exact = CATEGORY_SEARCH_SCOPES.find((s) => s.label === trimmed)
  if (exact) return exact.cateId
  const norm = trimmed.replace(/\s+/g, ' ')
  const byNorm = CATEGORY_SEARCH_SCOPES.find((s) => s.label.replace(/\s+/g, ' ') === norm)
  if (byNorm) return byNorm.cateId
  const leaf = trimmed.split('/').pop()?.trim()
  if (leaf) {
    const byLeaf = CATEGORY_SEARCH_SCOPES.find(
      (s) => s.label === leaf || s.label.endsWith(` / ${leaf}`),
    )
    if (byLeaf) return byLeaf.cateId
  }
  return undefined
}
