<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import {
  cancelDownloadTask,
  createDownloadTask,
  getConfig,
  getDownloadTaskSnapshots,
  listenDownloadTaskEvent,
  pauseDownloadTask,
  removeDownloadTaskRecord,
  resumeDownloadTask,
  type DownloadTaskEvent,
} from '../api'
import { buildProgressRow, type ProgressRow } from '../downloadProgress'

const rows = ref<ProgressRow[]>([])
const downloadFormat = ref<'JpegZipPack' | 'Server2Zip'>('Server2Zip')
const retryCount = ref(3)
const activeTab = ref<'queue' | 'failed' | 'completed'>('queue')
const selectedIds = ref<Set<number>>(new Set())
const expandedSeriesDirs = ref<Set<string>>(new Set())
const knownSeriesDirs = ref<Set<string>>(new Set())
let unlisten: (() => void) | null = null

function upsert(ev: DownloadTaskEvent) {
  const row = buildProgressRow(ev, downloadFormat.value, retryCount.value)
  const idx = rows.value.findIndex((r) => r.comic.id === ev.comic.id)
  if (idx >= 0) {
    const next = [...rows.value]
    next[idx] = row
    rows.value = next
  } else {
    rows.value = [...rows.value, row]
  }
}

async function refresh() {
  try {
    const snaps = await getDownloadTaskSnapshots()
    rows.value = snaps.map((ev) => buildProgressRow(ev, downloadFormat.value, retryCount.value))
  } catch {
    /* ignore */
  }
}

const queueRows = computed(() =>
  rows.value.filter((r) => r.state === 'Pending' || r.state === 'Downloading' || r.state === 'Paused'),
)

const failedRows = computed(() => rows.value.filter((r) => r.state === 'Failed'))

const completedRows = computed(() => rows.value.filter((r) => r.state === 'Completed'))

const visibleRows = computed(() => {
  if (activeTab.value === 'queue') return queueRows.value
  if (activeTab.value === 'failed') return failedRows.value
  return completedRows.value
})

const groupedVisibleRows = computed(() => {
  const seriesMap = new Map<string, ProgressRow[]>()
  const standalone: ProgressRow[] = []
  for (const row of visibleRows.value) {
    const dir = (row.seriesParentDir ?? '').trim()
    if (!dir) {
      standalone.push(row)
      continue
    }
    const list = seriesMap.get(dir) ?? []
    list.push(row)
    seriesMap.set(dir, list)
  }
  const seriesGroups = Array.from(seriesMap.entries()).map(([seriesParentDir, rows]) => ({
    seriesParentDir,
    rows,
  }))
  return { seriesGroups, standalone }
})

const allVisibleSelected = computed(() => {
  if (visibleRows.value.length === 0) return false
  return visibleRows.value.every((row) => selectedIds.value.has(row.comic.id))
})

function clearInvalidSelections() {
  const valid = new Set(visibleRows.value.map((row) => row.comic.id))
  const next = new Set<number>()
  for (const id of selectedIds.value) {
    if (valid.has(id)) next.add(id)
  }
  selectedIds.value = next
}

function isSeriesExpanded(seriesParentDir: string) {
  return expandedSeriesDirs.value.has(seriesParentDir)
}

function toggleSeriesExpanded(seriesParentDir: string) {
  const next = new Set(expandedSeriesDirs.value)
  if (next.has(seriesParentDir)) next.delete(seriesParentDir)
  else next.add(seriesParentDir)
  expandedSeriesDirs.value = next
}

function syncExpandedSeriesDirs() {
  const valid = new Set(groupedVisibleRows.value.seriesGroups.map((group) => group.seriesParentDir))
  const next = new Set<string>([...expandedSeriesDirs.value].filter((dir) => valid.has(dir)))
  const known = new Set<string>(knownSeriesDirs.value)
  for (const dir of valid) {
    if (known.has(dir)) continue
    next.add(dir)
    known.add(dir)
  }
  expandedSeriesDirs.value = next
  knownSeriesDirs.value = known
}

function toggleSelectOne(comicId: number, checked: boolean) {
  const next = new Set(selectedIds.value)
  if (checked) next.add(comicId)
  else next.delete(comicId)
  selectedIds.value = next
}

function toggleSelectAll(checked: boolean) {
  if (!checked) {
    selectedIds.value = new Set()
    return
  }
  selectedIds.value = new Set(visibleRows.value.map((row) => row.comic.id))
}

function onSelectAllChange(event: Event) {
  const checked = (event.target as HTMLInputElement).checked
  toggleSelectAll(checked)
}

function onSelectOneChange(comicId: number, event: Event) {
  const checked = (event.target as HTMLInputElement).checked
  toggleSelectOne(comicId, checked)
}

async function bulkPause() {
  for (const id of selectedIds.value) {
    await pauseDownloadTask(id).catch(() => undefined)
  }
}

async function bulkResume() {
  for (const id of selectedIds.value) {
    await resumeDownloadTask(id).catch(() => undefined)
  }
}

async function bulkCancel() {
  for (const id of selectedIds.value) {
    await cancelDownloadTask(id).catch(() => undefined)
  }
}

async function bulkRetry() {
  for (const id of selectedIds.value) {
    const row = rows.value.find((item) => item.comic.id === id)
    if (!row) continue
    await createDownloadTask({
      id: row.comic.id,
      title: row.comic.title,
      cover: row.comic.cover,
      category: row.comic.category,
      imageCount: row.comic.imageCount,
    }).catch(() => undefined)
  }
}

async function bulkRemove() {
  for (const id of selectedIds.value) {
    await removeDownloadTaskRecord(id).catch(() => undefined)
    rows.value = rows.value.filter((row) => row.comic.id !== id)
  }
  selectedIds.value = new Set()
}

function handleRemoveSingle(comicId: number) {
  void removeDownloadTaskRecord(comicId)
  rows.value = rows.value.filter((row) => row.comic.id !== comicId)
  const next = new Set(selectedIds.value)
  next.delete(comicId)
  selectedIds.value = next
}

watch(visibleRows, () => {
  clearInvalidSelections()
  syncExpandedSeriesDirs()
})

onMounted(async () => {
  try {
    const cfg = await getConfig()
    downloadFormat.value = cfg.downloadFormat
    retryCount.value = cfg.downloadRetryCount
  } catch {
    /* ignore */
  }
  await refresh()
  unlisten = await listenDownloadTaskEvent(upsert)
})

onUnmounted(() => {
  unlisten?.()
})
</script>

<template>
  <div class="dl-panel">
    <div class="tab-row">
      <button type="button" class="tab-btn" :class="{ on: activeTab === 'queue' }" @click="activeTab = 'queue'; clearInvalidSelections()">
        下載佇列
      </button>
      <button type="button" class="tab-btn" :class="{ on: activeTab === 'failed' }" @click="activeTab = 'failed'; clearInvalidSelections()">
        下載失敗
      </button>
      <button type="button" class="tab-btn" :class="{ on: activeTab === 'completed' }" @click="activeTab = 'completed'; clearInvalidSelections()">
        下載完成
      </button>
    </div>

    <div v-if="visibleRows.length > 0" class="toolbar">
      <label class="check-all">
        <input type="checkbox" :checked="allVisibleSelected" @change="onSelectAllChange" />
        全選
      </label>

      <div class="toolbar-actions">
        <button v-if="activeTab === 'queue'" type="button" class="mini" @click="bulkPause">暫停下載</button>
        <button v-if="activeTab === 'queue'" type="button" class="mini" @click="bulkResume">繼續下載</button>
        <button v-if="activeTab === 'queue'" type="button" class="mini" @click="bulkCancel">取消下載</button>
        <button v-if="activeTab === 'failed'" type="button" class="mini" @click="bulkRetry">重新下載</button>
        <button v-if="activeTab !== 'queue'" type="button" class="mini" @click="bulkRemove">清除紀錄</button>
      </div>
    </div>

    <p v-if="visibleRows.length === 0" class="empty">尚無下載任務</p>

    <section
      v-for="group in groupedVisibleRows.seriesGroups"
      :key="`group-${group.seriesParentDir}`"
      class="dl-group"
    >
      <button type="button" class="dl-group-head" @click="toggleSeriesExpanded(group.seriesParentDir)">
        <span class="dl-group-caret">{{ isSeriesExpanded(group.seriesParentDir) ? '▾' : '▸' }}</span>
        <span class="dl-group-title" :title="group.seriesParentDir">{{ group.seriesParentDir }}</span>
        <span class="dl-group-meta">{{ group.rows.length }} 項</span>
      </button>

      <div v-if="isSeriesExpanded(group.seriesParentDir)" class="dl-group-body">
        <article v-for="r in group.rows" :key="r.comic.id" class="dl-row">
          <div class="dl-title">{{ r.comic.title }}</div>
          <div class="dl-ind">{{ r.indicator }}</div>
          <div v-if="r.failureDetail" class="dl-err" :title="r.failureDetail">{{ r.failureDetail }}</div>
          <div v-if="!Number.isNaN(r.percentage)" class="dl-bar-wrap">
            <div class="dl-bar" :style="{ width: `${Math.min(100, r.percentage)}%` }" />
          </div>

          <div class="dl-footer">
            <div class="dl-select-row">
              <label>
                <input
                  type="checkbox"
                  :checked="selectedIds.has(r.comic.id)"
                  @change="onSelectOneChange(r.comic.id, $event)"
                />
                勾選
              </label>
            </div>

            <div class="dl-actions">
              <button
                v-if="r.state === 'Downloading'"
                type="button"
                class="mini"
                @click="pauseDownloadTask(r.comic.id)"
              >
                暫停
              </button>
              <button v-if="r.state === 'Paused'" type="button" class="mini" @click="resumeDownloadTask(r.comic.id)">
                繼續
              </button>
              <button
                v-if="r.state === 'Downloading' || r.state === 'Paused' || r.state === 'Pending'"
                type="button"
                class="mini"
                @click="cancelDownloadTask(r.comic.id)"
              >
                取消
              </button>
              <button
                v-if="r.state === 'Failed' || r.state === 'Completed' || r.state === 'Cancelled'"
                type="button"
                class="mini"
                @click="handleRemoveSingle(r.comic.id)"
              >
                移除
              </button>
            </div>
          </div>
        </article>
      </div>
    </section>

    <article v-for="r in groupedVisibleRows.standalone" :key="`single-${r.comic.id}`" class="dl-row">
      <div class="dl-title">{{ r.comic.title }}</div>
      <div class="dl-ind">{{ r.indicator }}</div>
      <div v-if="r.failureDetail" class="dl-err" :title="r.failureDetail">{{ r.failureDetail }}</div>
      <div v-if="!Number.isNaN(r.percentage)" class="dl-bar-wrap">
        <div class="dl-bar" :style="{ width: `${Math.min(100, r.percentage)}%` }" />
      </div>

      <div class="dl-footer">
        <div class="dl-select-row">
          <label>
            <input
              type="checkbox"
              :checked="selectedIds.has(r.comic.id)"
              @change="onSelectOneChange(r.comic.id, $event)"
            />
            勾選
          </label>
        </div>

        <div class="dl-actions">
          <button
            v-if="r.state === 'Downloading'"
            type="button"
            class="mini"
            @click="pauseDownloadTask(r.comic.id)"
          >
            暫停
          </button>
          <button v-if="r.state === 'Paused'" type="button" class="mini" @click="resumeDownloadTask(r.comic.id)">
            繼續
          </button>
          <button
            v-if="r.state === 'Downloading' || r.state === 'Paused' || r.state === 'Pending'"
            type="button"
            class="mini"
            @click="cancelDownloadTask(r.comic.id)"
          >
            取消
          </button>
          <button
            v-if="r.state === 'Failed' || r.state === 'Completed' || r.state === 'Cancelled'"
            type="button"
            class="mini"
            @click="handleRemoveSingle(r.comic.id)"
          >
            移除
          </button>
        </div>
      </div>
    </article>
  </div>
</template>

<style scoped>
.dl-panel {
  flex: 1;
  overflow-y: auto;
  padding: 6px 8px;
}
.tab-row {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 4px;
  margin-bottom: 6px;
}
.tab-btn {
  padding: 7px 4px;
  border: 1px solid #555;
  border-radius: 6px;
  background: #2a2a2a;
  color: #ddd;
  font-size: 11px;
}
.tab-btn.on {
  border-color: #3d6ef5;
  background: #20345d;
  color: #fff;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  margin-bottom: 6px;
}
.check-all {
  font-size: 10px;
  color: #bbb;
}
.toolbar-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 6px;
}
.empty {
  text-align: center;
  color: #999;
  font-size: 11px;
  padding: 16px;
}
.dl-group {
  margin-bottom: 6px;
  border: 1px solid #3b3b3b;
  border-radius: 6px;
  background: #202020;
}
.dl-group-head {
  width: 100%;
  border: none;
  background: transparent;
  color: #ddd;
  display: grid;
  grid-template-columns: 16px minmax(0, 1fr) auto;
  align-items: center;
  gap: 6px;
  padding: 7px 8px;
  text-align: left;
}
.dl-group-caret {
  color: #9c9c9c;
  font-size: 12px;
}
.dl-group-title {
  font-size: 11px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.dl-group-meta {
  font-size: 10px;
  color: #9f9f9f;
}
.dl-group-body {
  border-top: 1px solid #343434;
  padding: 6px;
}
.dl-row {
  padding: 8px;
  margin-bottom: 6px;
  border: 1px solid #444;
  border-radius: 6px;
  background: #252525;
}
.dl-group-body .dl-row:last-child {
  margin-bottom: 0;
}
.dl-title {
  font-size: 11px;
  font-weight: 600;
  margin-bottom: 2px;
  line-height: 1.3;
}
.dl-ind {
  font-size: 9px;
  color: #aaa;
  margin-bottom: 4px;
}
.dl-err {
  font-size: 9px;
  line-height: 1.35;
  color: #f08080;
  margin-bottom: 4px;
  word-break: break-word;
  white-space: pre-wrap;
}
.dl-bar-wrap {
  height: 3px;
  background: #333;
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 4px;
}
.dl-bar {
  height: 100%;
  background: #3d6ef5;
}
.dl-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
}
.dl-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 4px;
}
.dl-select-row {
  margin: 0;
  font-size: 10px;
  color: #bdbdbd;
  white-space: nowrap;
}
.mini {
  padding: 3px 7px;
  font-size: 9px;
  border: 1px solid #555;
  border-radius: 4px;
  background: #333;
  color: #eee;
}
</style>
