import type { ComicInSearch, SearchResult } from './api'
import type { SearchSortOrder } from './searchUtils'

export type RankingPeriod = 'Day' | 'Week' | 'Month' | 'Year'

export type SearchSource =
  | { type: 'keyword'; cateId?: number }
  | { type: 'tag'; source: 'name' | 'link'; cateId?: number }
  | { type: 'category'; cateId: number }
  | { type: 'albums'; list: 'home' | 'albums' }
  | { type: 'ranking'; period: RankingPeriod; cateId: number | null }

export interface SearchResultTabState {
  id: string
  title: string
  keywordOrComicLinkInput: string
  tagOrLinkInput: string
  activeTagSearchSource: 'name' | 'link'
  searchSource: SearchSource
  searchScopeCategory: { cateId: number; label: string } | null
  activeTagLabel: string
  activeCategoryLabel: string
  rankingPeriod: RankingPeriod
  viewPage: number
  allSearchComics: ComicInSearch[]
  sortedComics: ComicInSearch[]
  visibleComics: ComicInSearch[]
  pageCacheEntries: [number, SearchResult][]
  sortOrder: SearchSortOrder
  catalogAnalysisEntries: [number, string][]
  totalServerPagesHint: number
  totalCountHint: number
  totalCountRefined?: boolean
  lastCommittedViewPage: number
  searchSession: number
  coverLoadLimit: number
  searchResult?: SearchResult
  listScrollTop: number
  scopedOfflineMatches?: ComicInSearch[]
  globalSnapshotMetaId?: string
}
