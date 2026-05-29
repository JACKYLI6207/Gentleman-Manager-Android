<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { getConfig, readKoreanTxtCatalog, type ComicInSearch } from '../api'
import {
  analyzeKoreanWebtoon,
  defaultCheckedIds,
  type ClassifiedKoreanItem,
  type KoreanDownloadStrategy,
} from '../koreanWebtoon'
import { analyzeKoreanTxtDuplicates, type KoreanTxtDuplicateAnalysis } from '../koreanTxtDuplicate'

const props = defineProps<{
  showing: boolean
  comics: ComicInSearch[]
  tagLabel: string
}>()

const emit = defineEmits<{
  'update:showing': [showing: boolean]
  confirm: [
    payload: {
      selectedItems: ClassifiedKoreanItem[]
      rangeMin: number
      rangeMax: number
      tagLabel: string
    },
  ]
}>()

const strategy = ref<KoreanDownloadStrategy>('episodes')
const checkedIds = ref<Set<number>>(new Set())
const txtDuplicateLoading = ref(false)
const txtDuplicateError = ref('')
const txtDuplicateAnalysis = ref<KoreanTxtDuplicateAnalysis | null>(null)

const analysis = computed(() => analyzeKoreanWebtoon(props.comics, props.tagLabel))
const previewItems = computed(() =>
  strategy.value === 'anthology' ? analysis.value.anthologies : analysis.value.episodes,
)
const episodesDisabled = computed(() => analysis.value.episodes.length === 0)
const anthologyDisabled = computed(() => analysis.value.anthologies.length === 0)

function applyDefaultChecks() {
  checkedIds.value = new Set(defaultCheckedIds(analysis.value, strategy.value))
}

function closeDialog() {
  emit('update:showing', false)
}

function toggleItem(id: number, checked: boolean) {
  const next = new Set(checkedIds.value)
  if (checked) next.add(id)
  else next.delete(id)
  checkedIds.value = next
}

async function runTxtDuplicateAnalysis() {
  txtDuplicateAnalysis.value = null
  txtDuplicateError.value = ''
  try {
    const config = await getConfig()
    if (!config.koreanTxtDuplicateCheckEnabled) return
    const catalogDir = (config.koreanTxtCatalogDir ?? '').trim()
    if (!catalogDir) return
    txtDuplicateLoading.value = true
    const lines = await readKoreanTxtCatalog(catalogDir)
    txtDuplicateAnalysis.value = analyzeKoreanTxtDuplicates(
      analysis.value.tagLabel,
      props.comics,
      lines,
    )
  } catch (err) {
    txtDuplicateError.value = err instanceof Error ? err.message : String(err)
  } finally {
    txtDuplicateLoading.value = false
  }
}

function confirmSelection() {
  const selectedItems = previewItems.value.filter((item) => checkedIds.value.has(item.comic.id))
  if (selectedItems.length === 0) return
  emit('confirm', {
    selectedItems,
    rangeMin: analysis.value.rangeMin,
    rangeMax: analysis.value.rangeMax,
    tagLabel: analysis.value.tagLabel,
  })
  emit('update:showing', false)
}

watch(
  () => props.showing,
  (show) => {
    if (!show) {
      txtDuplicateLoading.value = false
      txtDuplicateError.value = ''
      txtDuplicateAnalysis.value = null
      return
    }
    if (analysis.value.episodes.length > 0) strategy.value = 'episodes'
    else if (analysis.value.anthologies.length > 0) strategy.value = 'anthology'
    applyDefaultChecks()
    void runTxtDuplicateAnalysis()
  },
  { immediate: true },
)

watch(strategy, () => applyDefaultChecks())
</script>

<template>
  <div
    v-if="showing"
    class="k-dialog-overlay"
    @click.self="closeDialog"
    @touchstart.self="closeDialog"
  >
    <div class="k-dialog" @click.stop @touchstart.stop>
      <div class="k-header">
        <p class="k-title">韓漫下載模式 · {{ analysis.tagLabel }}</p>
        <button type="button" class="k-close" @click.stop="closeDialog">×</button>
      </div>

      <p class="k-subtitle">
        話數範圍：{{ analysis.rangeMin }}～{{ analysis.rangeMax }}{{ analysis.marksComplete ? '（含完結）' : '' }}
      </p>

      <p v-if="analysis.coherenceWarning" class="k-warning">{{ analysis.coherenceWarning }}</p>

      <p v-if="txtDuplicateLoading" class="k-info">正在比對韓漫 TXT 收藏列表…</p>
      <p v-else-if="txtDuplicateError" class="k-error">{{ txtDuplicateError }}</p>
      <div
        v-else-if="txtDuplicateAnalysis && txtDuplicateAnalysis.catalogLineCount > 0"
        class="k-info-card"
      >
        <template v-if="txtDuplicateAnalysis.seriesMatches.length > 0">
          <p>
            已在 TXT 列表找到 {{ txtDuplicateAnalysis.seriesMatches.length }} 筆可能重複（共讀取
            {{ txtDuplicateAnalysis.catalogLineCount }} 行）
          </p>
          <ul class="k-dup-list">
            <li
              v-for="(line, idx) in txtDuplicateAnalysis.seriesMatches.slice(0, 8)"
              :key="`${idx}-${line}`"
            >
              {{ line }}
            </li>
          </ul>
          <p v-if="txtDuplicateAnalysis.seriesMatches.length > 8" class="k-dup-more">
            …另有 {{ txtDuplicateAnalysis.seriesMatches.length - 8 }} 筆
          </p>
        </template>
        <p v-else>TXT 列表中未發現與「{{ analysis.tagLabel }}」重複的項目</p>
      </div>

      <div class="k-mode">
        <label class="k-radio">
          <input
            type="radio"
            name="korean-strategy"
            value="episodes"
            :checked="strategy === 'episodes'"
            :disabled="episodesDisabled"
            @change="strategy = 'episodes'"
          />
          分集連載（{{ analysis.episodes.length }} 項，依話數批次下載）
        </label>
        <label class="k-radio">
          <input
            type="radio"
            name="korean-strategy"
            value="anthology"
            :checked="strategy === 'anthology'"
            :disabled="anthologyDisabled"
            @change="strategy = 'anthology'"
          />
          完整合集（{{ analysis.anthologies.length }} 項，單檔收齊全篇）
        </label>
      </div>

      <p class="k-folder-hint">
        將下載至新資料夾：{{ analysis.tagLabel }}-{{ analysis.rangeMin }}~{{ analysis.rangeMax }}-完
      </p>

      <div class="k-list">
        <div v-if="previewItems.length === 0" class="k-empty">此標籤下沒有可辨識的韓漫分集或合集</div>
        <label v-for="item in previewItems" :key="item.comic.id" class="k-item">
          <input
            type="checkbox"
            :checked="checkedIds.has(item.comic.id)"
            @change="toggleItem(item.comic.id, ($event.target as HTMLInputElement).checked)"
          />
          <div class="k-item-main">
            <p class="k-item-title">{{ item.comic.title }}</p>
            <p class="k-item-sub">
              {{
                item.rangeStart === item.rangeEnd
                  ? `第 ${item.rangeStart} 話`
                  : `第 ${item.rangeStart}-${item.rangeEnd} 話`
              }}
              · {{ item.imageCount }} 張{{ item.comic.isDownloaded ? ' · 已下載' : '' }}
            </p>
            <p
              v-if="txtDuplicateAnalysis?.itemMessages.get(item.comic.id)"
              class="k-item-dup"
            >
              {{ txtDuplicateAnalysis?.itemMessages.get(item.comic.id) }}
            </p>
          </div>
        </label>
      </div>

      <div class="k-actions">
        <button type="button" class="k-btn k-btn--ghost" @click.stop="closeDialog">取消</button>
        <button
          type="button"
          class="k-btn k-btn--ok"
          :disabled="checkedIds.size === 0"
          @click.stop="confirmSelection"
        >
          加入下載佇列
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.k-dialog-overlay {
  position: fixed;
  inset: 0;
  z-index: 1400;
  background: rgba(0, 0, 0, 0.55);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 12px;
}

.k-dialog {
  width: min(92vw, 560px);
  max-height: 86vh;
  border-radius: 10px;
  background: #252525;
  border: 1px solid #3d3d3d;
  display: flex;
  flex-direction: column;
  padding: 12px;
  gap: 8px;
}

.k-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.k-title {
  margin: 0;
  color: #f3f3f3;
  font-size: 20px;
  font-weight: 700;
}

.k-close {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: #bdbdbd;
  font-size: 20px;
}

.k-subtitle {
  margin: 0;
  color: #d6d6d6;
  font-size: 14px;
}

.k-warning {
  margin: 0;
  padding: 8px 10px;
  border-radius: 6px;
  background: #4f3c14;
  color: #ffd37a;
  font-size: 13px;
}

.k-info,
.k-error {
  margin: 0;
  font-size: 13px;
}

.k-info {
  color: #9ec8ff;
}

.k-error {
  color: #ff9f9f;
}

.k-info-card {
  border: 1px solid #2f705f;
  background: #1f4f42;
  color: #d6fff1;
  border-radius: 8px;
  padding: 10px;
  font-size: 13px;
}

.k-info-card p {
  margin: 0;
}

.k-dup-list {
  margin: 8px 0 0;
  padding: 0 0 0 4px;
  list-style: none;
  max-height: 120px;
  overflow-y: auto;
}

.k-dup-list li {
  margin: 4px 0;
  line-height: 1.35;
  word-break: break-word;
  color: #e8fff8;
}

.k-dup-more {
  margin: 6px 0 0;
  font-size: 12px;
  opacity: 0.85;
}

.k-mode {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.k-radio {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #dbdbdb;
  font-size: 14px;
}

.k-folder-hint {
  margin: 0;
  color: #c6c6c6;
  font-size: 12px;
}

.k-list {
  border: 1px solid #474747;
  border-radius: 8px;
  padding: 8px;
  max-height: 42vh;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.k-empty {
  color: #a3a3a3;
  font-size: 13px;
  text-align: center;
  padding: 10px 0;
}

.k-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.k-item-main {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.k-item-title {
  margin: 0;
  color: #efefef;
  font-size: 16px;
  line-height: 1.25;
}

.k-item-sub {
  margin: 0;
  color: #b4b4b4;
  font-size: 13px;
}

.k-item-dup {
  margin: 0;
  color: #f2bd7a;
  font-size: 12px;
}

.k-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

.k-btn {
  min-width: 88px;
  border: none;
  border-radius: 6px;
  padding: 8px 12px;
  font-size: 14px;
}

.k-btn--ghost {
  background: #3a3a3a;
  color: #d8d8d8;
}

.k-btn--ok {
  background: #2d7ef8;
  color: #fff;
}

.k-btn--ok:disabled {
  opacity: 0.45;
}
</style>
