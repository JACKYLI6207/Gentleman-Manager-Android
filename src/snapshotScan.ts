import { resolveCateIdFromLabel } from './categories'
import { formatInvokeError } from './invokeError'
import {
  browseAlbumsList,
  browseByCategory,
  getConfig,
  snapshotScanApplyUpdatePage,
  snapshotScanBegin,
  snapshotScanDispose,
  snapshotScanFinalize,
  snapshotScanMergeBatch,
  type ComicInSearch,
  type SearchResult,
  type SnapshotExistingLoadMode,
  type SnapshotResumeCandidate,
} from './api'

export type SnapshotResumeStrategy = 'page' | 'idUpdate'

export type SnapshotScanProgress = {
  label: string
  current: number
  total: number
  matchedCount: number
  phase: string
}

type SnapshotMetaFields = {
  id: string
  savedAt: string
  totalCount: number
  totalPages: number
  scanCompletionPercent: number
  scanCompletedPages: number
  scanCompletedPageRanges: { start: number; end: number }[]
  scanTargetCateId?: number
  scanTargetLabel?: string
}

let scanCancelled = false

export function cancelSnapshotScan() {
  scanCancelled = true
}

export function resetSnapshotScanCancel() {
  scanCancelled = false
}

function shouldStop() {
  return scanCancelled
}

function sleep(ms: number) {
  return new Promise<void>((r) => setTimeout(r, ms))
}

function randomInt(min: number, max: number) {
  return Math.floor(Math.random() * (max - min + 1)) + min
}

function clampPercent(n: number) {
  return Math.max(0, Math.min(100, Math.round(n)))
}

export function snapshotCompletionPercent(c: SnapshotResumeCandidate): number {
  if (c.totalPages > 0 && c.scanCompletedPages > 0) {
    const p = (Math.min(c.scanCompletedPages, c.totalPages) / c.totalPages) * 100
    return c.scanCompletedPages >= c.totalPages ? 100 : Math.min(99, clampPercent(p))
  }
  return clampPercent(c.scanCompletionPercent ?? 100)
}

export function isSnapshotScanComplete(c: SnapshotResumeCandidate): boolean {
  if (c.totalPages <= 0) return false
  if (c.scanCompletedPages > 0) return c.scanCompletedPages >= c.totalPages
  return snapshotCompletionPercent(c) >= 100
}

/** 優先顯示檔案修改時間（實際存檔），其次 JSON 內 savedAt（掃描當下寫入的 meta） */
export function snapshotFileDate(c: SnapshotResumeCandidate): Date | null {
  if (c.modifiedMs != null && c.modifiedMs > 0) {
    const d = new Date(c.modifiedMs)
    if (!Number.isNaN(d.getTime())) return d
  }
  if (c.savedAt) {
    const d = new Date(c.savedAt)
    if (!Number.isNaN(d.getTime())) return d
  }
  return null
}

export function snapshotDisplayLabel(c: SnapshotResumeCandidate): string {
  const label = c.label.trim() || '未命名分類'
  const d = snapshotFileDate(c)
  if (d) {
    return `${label}快照 ${d.toLocaleString()}`
  }
  return `${label}快照`
}

export type SnapshotScanFailure = {
  label: string
  message: string
}

export type SnapshotScanTargetKind = 'category' | 'albums'

type ResolvedSnapshotScanTarget =
  | { kind: 'albums' }
  | { kind: 'category'; cateId: number }

/** 分類快照需 cateId；官網「更新」列表為 albums，無分類 ID */
export function resolveSnapshotScanTarget(
  c: SnapshotResumeCandidate,
): ResolvedSnapshotScanTarget | { error: string } {
  const kindRaw = (c.scanTargetKind ?? '').trim().toLowerCase()
  const label = c.label.trim()
  if (kindRaw === 'albums' || (!kindRaw && label === '更新')) {
    return { kind: 'albums' }
  }
  let cateId = c.cateId
  if (cateId == null || cateId < 0) {
    cateId = resolveCateIdFromLabel(label)
  }
  if (cateId == null || cateId < 0) {
    return {
      error: `${label || '此快照'} 缺少分類 ID，且無法從名稱對應官網分類。請在 PC 版重新保存（含 scanTargetCateId），或確認標籤如「同人誌 / 漢化」。`,
    }
  }
  return { kind: 'category', cateId }
}

export function snapshotTargetKindLabel(c: SnapshotResumeCandidate): string {
  const resolved = resolveSnapshotScanTarget(c)
  if ('error' in resolved) return '未知'
  return resolved.kind === 'albums' ? '官網更新' : `分類 ID ${resolved.cateId}`
}

function pageRangesFromSet(pages: Set<number>): { start: number; end: number }[] {
  if (pages.size === 0) return []
  const sorted = [...pages].sort((a, b) => a - b)
  const ranges: { start: number; end: number }[] = []
  let start = sorted[0]!
  let end = start
  for (let i = 1; i < sorted.length; i++) {
    const p = sorted[i]!
    if (p === end + 1) {
      end = p
    } else {
      ranges.push({ start, end })
      start = p
      end = p
    }
  }
  ranges.push({ start, end })
  return ranges
}

function snapshotTimestamp() {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}_${pad(d.getMonth() + 1)}_${pad(d.getDate())}_${pad(d.getHours())}_${pad(d.getMinutes())}_${pad(d.getSeconds())}`
}

function safeSnapshotFileName(label: string) {
  const safe = label.replace(/[\\/:*?"<>|]/g, '_').replace(/\s+/g, ' ').trim()
  return `${safe || 'snapshot'} ${snapshotTimestamp()}.gm-snapshot.json`
}

function highestUncompletedPage(totalPages: number, completed: Set<number>): number {
  for (let p = totalPages; p >= 1; p--) {
    if (!completed.has(p)) return p
  }
  return 1
}

function metaFromRaw(raw: Record<string, unknown>, fallbackPages: number): SnapshotMetaFields {
  return {
    id: String(raw.id ?? crypto.randomUUID()),
    savedAt: String(raw.savedAt ?? new Date().toISOString()),
    totalCount: Number(raw.totalCount ?? 0),
    totalPages: Number(raw.totalPages ?? fallbackPages),
    scanCompletionPercent: Number(raw.scanCompletionPercent ?? 0),
    scanCompletedPages: Number(raw.scanCompletedPages ?? 0),
    scanCompletedPageRanges: Array.isArray(raw.scanCompletedPageRanges)
      ? (raw.scanCompletedPageRanges as { start: number; end: number }[])
      : [],
    scanTargetCateId: raw.scanTargetCateId != null ? Number(raw.scanTargetCateId) : undefined,
    scanTargetLabel: raw.scanTargetLabel != null ? String(raw.scanTargetLabel) : undefined,
  }
}

async function fetchSnapshotPage(
  target: ResolvedSnapshotScanTarget,
  page: number,
): Promise<SearchResult | undefined> {
  try {
    if (target.kind === 'albums') {
      return await browseAlbumsList(page)
    }
    return await browseByCategory(target.cateId, page)
  } catch {
    return undefined
  }
}

/** 官網列表每頁 20 本（與 PC SERVER_PAGE_SIZE 相同） */
const SERVER_PAGE_SIZE = 20

const DEFAULT_UPDATE_DUP_STOP_THRESHOLD = 20
/** 舊 ID 重複超過設定值後，須再連續 N 頁無新增才允許停止更新掃描 */
const UPDATE_ZERO_ADD_STOP_STREAK = 2

function normalizeUpdateDupStopThreshold(value: number | null | undefined): number {
  const n = Math.floor(Number(value ?? DEFAULT_UPDATE_DUP_STOP_THRESHOLD))
  if (!Number.isFinite(n) || n < 0) return DEFAULT_UPDATE_DUP_STOP_THRESHOLD
  return Math.min(n, 9999)
}

/** 保守模式：僅模擬手點逐頁請求（無並行），每頁間隔 0.5～1.5 秒 */
const TAP_DELAY_MS = { min: 500, max: 1500 } as const

function missingPagesInRange(completed: Set<number>, from: number, to: number): number[] {
  const missing: number[] = []
  for (let p = from; p <= to; p++) {
    if (!completed.has(p)) missing.push(p)
  }
  return missing
}

function highestCompletedPage(completed: Set<number>): number {
  if (completed.size === 0) return 0
  return Math.max(...completed)
}

/** 已掃範圍 1..最高頁 須連續無缺口；任一頁缺漏皆不可寫檔 */
function contiguousMissingPages(completed: Set<number>): number[] {
  const through = highestCompletedPage(completed)
  if (through <= 0) return []
  return missingPagesInRange(completed, 1, through)
}

function pendingUnscannedPages(completed: Set<number>, retryQueue: Set<number>): number[] {
  return [...new Set([...retryQueue, ...contiguousMissingPages(completed)])].sort((a, b) => a - b)
}

function isUsableSnapshotPageResult(
  result: SearchResult | undefined,
  requestedPage: number,
  totalPages: number,
  failAttempts: number,
): boolean {
  if (!result) return false
  // 官網回傳 HTML 的 .thispage 須與請求頁碼一致，否則可能寫入錯頁、造成整頁缺口
  if (result.currentPage !== requestedPage) return false
  if (result.comics.length === 0) return false
  if (requestedPage >= totalPages) return true
  if (result.comics.length >= SERVER_PAGE_SIZE) return true
  return failAttempts >= 4 && result.comics.length >= 1
}

export async function scanCategorySnapshot(
  categoryDir: string,
  candidate: SnapshotResumeCandidate,
  options: {
    mode: 'resume' | 'update'
    strategy: SnapshotResumeStrategy
    onProgress?: (p: SnapshotScanProgress) => void
  },
): Promise<{ ok: boolean; message: string }> {
  const target = resolveSnapshotScanTarget(candidate)
  if ('error' in target && typeof target.error === 'string') {
    return { ok: false, message: target.error }
  }

  const label = candidate.label.trim() || '未命名分類'
  const scanTargetKind = target.kind
  const cateIdForSession = target.kind === 'category' ? target.cateId : -1
  const report = (phase: string, current: number, total: number, matchedCount: number) => {
    options.onProgress?.({ label, current, total, matchedCount, phase })
  }

  let sessionId = ''
  let totalCount = 0

  try {
    report('讀取現有快照…', 0, 1, 0)
    const needExisting = options.mode === 'resume' || options.mode === 'update'
    const firstPage = await fetchSnapshotPage(target, 1)
    if (!firstPage || shouldStop()) {
      return { ok: false, message: shouldStop() ? '已取消' : `${label} 取得第 1 頁失敗` }
    }

    const totalPages = Math.max(1, firstPage.totalPage)
    const existingLoad: SnapshotExistingLoadMode =
      needExisting && options.mode === 'update' ? 'idsOnly' : 'full'
    const begin = await snapshotScanBegin(
      needExisting ? candidate.filePath : null,
      cateIdForSession,
      label,
      totalPages,
      scanTargetKind,
      existingLoad,
    )
    sessionId = begin.sessionId
    totalCount = begin.existingCount

    if (options.mode === 'resume' && begin.existingCount === 0 && needExisting) {
      return { ok: false, message: `找不到要接續的 ${label} 快照內容` }
    }
    if (options.mode === 'update' && begin.existingCount === 0) {
      return { ok: false, message: `找不到要更新的 ${label} 快照內容` }
    }

    const meta = metaFromRaw(begin.meta, totalPages)
    const completedPages = new Set<number>()
    if (options.mode === 'resume') {
      const done = Math.min(meta.scanCompletedPages, totalPages)
      for (let p = 1; p <= done; p++) completedPages.add(p)
      for (const range of meta.scanCompletedPageRanges) {
        for (let p = range.start; p <= range.end; p++) {
          if (p >= 1 && p <= totalPages) completedPages.add(p)
        }
      }
    }

    let batchComics: ComicInSearch[] = []
    let consecutiveFailedAttempts = 0
    const retryQueuedPages = new Set<number>()

    function queueRetryPage(page: number) {
      if (page >= 1 && page <= totalPages && !completedPages.has(page)) {
        consecutiveFailedAttempts += 1
        retryQueuedPages.add(page)
      }
    }

    async function waitConsecutiveFailureCooldown(phaseLabel: string, pageHint: number): Promise<boolean> {
      if (consecutiveFailedAttempts <= 3) return true
      for (let remaining = 20; remaining >= 1; remaining -= 1) {
        if (shouldStop()) return false
        report(
          `${phaseLabel}：連續失敗 ${consecutiveFailedAttempts} 次，${remaining} 秒後重試第 ${pageHint} 頁…`,
          pageHint,
          totalPages,
          totalCount,
        )
        await sleep(1000)
      }
      consecutiveFailedAttempts = 0
      return !shouldStop()
    }

    const pushBatchToRust = async () => {
      if (batchComics.length === 0) return
      const merged = await snapshotScanMergeBatch(sessionId, batchComics)
      batchComics = []
      totalCount = merged.totalCount
    }

    const appendPage = async (page: number, result: SearchResult) => {
      if (completedPages.has(page)) return
      batchComics.push(...result.comics)
      completedPages.add(page)
      consecutiveFailedAttempts = 0
      if (batchComics.length >= 80) {
        await pushBatchToRust()
      }
      report('掃描中', completedPages.size, totalPages, totalCount)
    }

    async function fetchWebsitePageWithRetry(
      page: number,
      phaseLabel: string,
    ): Promise<SearchResult | undefined> {
      let pageFailAttempts = 0
      while (!shouldStop()) {
        if (page !== 1) {
          await sleep(randomInt(TAP_DELAY_MS.min, TAP_DELAY_MS.max))
        }
        const result = await fetchSnapshotPage(target, page)
        if (isUsableSnapshotPageResult(result, page, totalPages, pageFailAttempts)) {
          retryQueuedPages.delete(page)
          consecutiveFailedAttempts = 0
          return result
        }
        pageFailAttempts += 1
        if (shouldStop()) return undefined
        queueRetryPage(page)
        if (!(await waitConsecutiveFailureCooldown(phaseLabel, page))) {
          return undefined
        }
      }
      return undefined
    }

    if (options.mode === 'update') {
      const updateDupStopThreshold = normalizeUpdateDupStopThreshold(
        (await getConfig()).snapshotUpdateDuplicateStopCount,
      )
      let duplicateHits = 0
      let addedTotal = 0
      let updateInterrupted = false
      let pastDupThreshold = false
      let zeroAddStreak = 0

      const applyUpdatePageResult = async (page: number, comics: ComicInSearch[]) => {
        const r = await snapshotScanApplyUpdatePage(sessionId, comics, duplicateHits)
        duplicateHits = r.duplicateHits
        addedTotal += r.addedCount
        totalCount = r.totalCount
        completedPages.add(page)
        report('更新掃描', page, totalPages, totalCount)
        return r
      }

      if (!isUsableSnapshotPageResult(firstPage, 1, totalPages, 0)) {
        return {
          ok: false,
          message: `${label} 官網第 1 頁回應異常（頁碼 ${firstPage?.currentPage ?? '?'} 或筆數不符）`,
        }
      }

      await applyUpdatePageResult(1, firstPage.comics)

      // 逐頁 2→N 依序請求，不可跳號；duplicateHits 不得作為 for 條件（否則會漏中間任一页）
      for (let page = 2; page <= totalPages && !shouldStop(); page++) {
        const result = await fetchWebsitePageWithRetry(page, '更新掃描')
        if (result === undefined) {
          updateInterrupted = true
          break
        }
        const r = await applyUpdatePageResult(page, result.comics)

        if (duplicateHits > updateDupStopThreshold) pastDupThreshold = true
        if (pastDupThreshold) {
          zeroAddStreak = r.addedCount <= 0 ? zeroAddStreak + 1 : 0
          // 須在本頁成功寫入後才判斷停止，確保每一頁都實際請求過
          if (zeroAddStreak >= UPDATE_ZERO_ADD_STOP_STREAK) break
        }
      }

      while (retryQueuedPages.size > 0 && !shouldStop()) {
        const pages = [...retryQueuedPages].sort((a, b) => a - b)
        retryQueuedPages.clear()
        for (const p of pages) {
          if (shouldStop()) break
          const result = await fetchWebsitePageWithRetry(p, '更新重試')
          if (result === undefined) {
            queueRetryPage(p)
            updateInterrupted = true
            continue
          }
          await applyUpdatePageResult(p, result.comics)
        }
        if (retryQueuedPages.size > 0) {
          const hint = [...retryQueuedPages].sort((a, b) => a - b)[0] ?? 1
          if (!(await waitConsecutiveFailureCooldown('更新重試', hint))) {
            updateInterrupted = true
            break
          }
        }
      }

      if (shouldStop()) return { ok: false, message: '已取消' }

      const pending = pendingUnscannedPages(completedPages, retryQueuedPages)
      if (updateInterrupted || pending.length > 0) {
        return {
          ok: false,
          message:
            pending.length > 0
              ? `${label} 更新未完成：官網第 ${pending.join('、')} 頁未掃描，未寫入快照（請重試）`
              : `${label} 更新中斷，未寫入快照`,
        }
      }

      if (addedTotal === 0) {
        return { ok: true, message: `${label} 已是最新` }
      }

      report('寫入檔案…', totalPages, totalPages, totalCount)
      await snapshotScanFinalize({
        sessionId,
        categoryDirectory: categoryDir,
        fileName: safeSnapshotFileName(`${label}快照`),
        previousSnapshotPath: candidate.filePath,
        totalPages,
        scanCompletionPercent: 100,
        scanCompletedPages: totalPages,
        scanCompletedPageRanges: totalPages > 0 ? [{ start: 1, end: totalPages }] : [],
      })

      return {
        ok: true,
        message: addedTotal > 0 ? `${label} 已更新（新增 ${addedTotal} 本）` : `${label} 已是最新`,
      }
    }

    /** 從末頁往起點逐頁請求，失敗不跳頁、不標完成 */
    async function collectPacedPages(startPage: number, endPage: number): Promise<boolean> {
      for (let p = endPage; p >= startPage; p--) {
        if (shouldStop()) return false
        const result = await fetchWebsitePageWithRetry(p, '接續掃描')
        if (result === undefined) return false
        await appendPage(p, result)
      }
      return !shouldStop()
    }

    async function drainRetryQueue(phaseLabel: string): Promise<boolean> {
      while (retryQueuedPages.size > 0 && !shouldStop()) {
        const pages = [...retryQueuedPages].sort((a, b) => b - a)
        retryQueuedPages.clear()
        for (const p of pages) {
          if (shouldStop()) return false
          const result = await fetchWebsitePageWithRetry(p, phaseLabel)
          if (result === undefined) {
            queueRetryPage(p)
            continue
          }
          await appendPage(p, result)
        }
        await pushBatchToRust()
        if (!(await waitConsecutiveFailureCooldown(phaseLabel, pages[0] ?? 1))) return false
      }
      return !shouldStop()
    }

    const resumeStartPage = highestUncompletedPage(totalPages, completedPages)

    if (resumeStartPage > 1) {
      await collectPacedPages(2, resumeStartPage)
    }
    if (!shouldStop() && !completedPages.has(1)) {
      await appendPage(1, firstPage)
    }
    await pushBatchToRust()
    if (!shouldStop()) {
      await drainRetryQueue('接續重試')
    }

    await pushBatchToRust()

    if (shouldStop()) return { ok: false, message: '已取消' }

    const pending = pendingUnscannedPages(completedPages, retryQueuedPages)
    if (pending.length > 0) {
      return {
        ok: false,
        message: `${label} 接續未完成：官網第 ${pending.join('、')} 頁未掃描，未寫入快照`,
      }
    }

    if (totalCount === 0) {
      return { ok: false, message: `${label} 沒有可保存的資料` }
    }

    const completed = completedPages.size
    const finalPct =
      totalPages > 0 && completed < totalPages
        ? Math.min(99, (completed / totalPages) * 100)
        : 100

    report('寫入檔案…', totalPages, totalPages, totalCount)
    await snapshotScanFinalize({
      sessionId,
      categoryDirectory: categoryDir,
      fileName: safeSnapshotFileName(`${label}快照`),
      previousSnapshotPath: candidate.filePath,
      totalPages,
      scanCompletionPercent: clampPercent(finalPct),
      scanCompletedPages: completed,
      scanCompletedPageRanges: pageRangesFromSet(completedPages),
    })

    return { ok: true, message: `已保存 ${label} 快照` }
  } catch (e) {
    const msg = formatInvokeError(e)
    return { ok: false, message: msg || `${label} 快照掃描失敗` }
  } finally {
    if (sessionId) {
      try {
        await snapshotScanDispose(sessionId)
      } catch {
        /* ignore */
      }
    }
  }
}

export async function runSnapshotResumeQueue(
  categoryDir: string,
  candidates: SnapshotResumeCandidate[],
  selectedPaths: string[],
  strategy: SnapshotResumeStrategy,
  onProgress?: (p: SnapshotScanProgress & { index: number; totalItems: number }) => void,
  onItemDone?: (message: string) => void,
): Promise<{ completed: number; failed: number; failures: SnapshotScanFailure[] }> {
  resetSnapshotScanCancel()
  const selected = selectedPaths
    .map((path) => candidates.find((c) => c.filePath === path))
    .filter((c): c is SnapshotResumeCandidate => c !== undefined)

  let completed = 0
  let failed = 0
  const failures: SnapshotScanFailure[] = []
  for (let i = 0; i < selected.length; i++) {
    if (shouldStop()) break
    const c = selected[i]!
    const mode =
      isSnapshotScanComplete(c) || strategy === 'idUpdate' ? 'update' : 'resume'
    onProgress?.({
      label: c.label,
      current: 0,
      total: 1,
      matchedCount: 0,
      phase: `排隊 ${i + 1}/${selected.length}`,
      index: i + 1,
      totalItems: selected.length,
    })
    const result = await scanCategorySnapshot(categoryDir, c, {
      mode,
      strategy,
      onProgress: (p) =>
        onProgress?.({
          ...p,
          index: i + 1,
          totalItems: selected.length,
        }),
    })
    if (result.ok) {
      completed++
      onItemDone?.(result.message)
    } else {
      failed++
      const itemLabel = c.label.trim() || '未命名分類'
      const errText = formatInvokeError(result.message)
      failures.push({ label: itemLabel, message: errText })
      onItemDone?.(`${itemLabel}：${errText}`)
      if (result.message === '已取消') break
    }
  }
  return { completed, failed, failures }
}
