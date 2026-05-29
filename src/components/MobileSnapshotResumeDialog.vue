<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { SnapshotResumeCandidate } from '../api'
import {
  isSnapshotScanComplete,
  snapshotCompletionPercent,
  snapshotDisplayLabel,
  snapshotFileDate,
  type SnapshotResumeStrategy,
} from '../snapshotScan'

const props = defineProps<{
  showing: boolean
  candidates: SnapshotResumeCandidate[]
  loading?: boolean
}>()

const emit = defineEmits<{
  'update:showing': [showing: boolean]
  confirm: [payload: { selectedPaths: string[]; strategy: SnapshotResumeStrategy }]
}>()

const selectedPaths = ref<Set<string>>(new Set())
const strategy = ref<SnapshotResumeStrategy>('page')

const allSelected = computed(() => {
  const n = props.candidates.length
  return n > 0 && selectedPaths.value.size === n
})

const selectIndeterminate = computed(() => {
  const s = selectedPaths.value.size
  const n = props.candidates.length
  return s > 0 && s < n
})

const hasIncompleteSelection = computed(() =>
  props.candidates.some((c) => selectedPaths.value.has(c.filePath) && !isSnapshotScanComplete(c)),
)

watch(
  () => props.showing,
  (show) => {
    if (!show) return
    selectedPaths.value = new Set(props.candidates.map((c) => c.filePath))
    strategy.value = 'page'
  },
  { immediate: true },
)

function closeDialog() {
  emit('update:showing', false)
}

function toggleAll(checked: boolean) {
  selectedPaths.value = checked
    ? new Set(props.candidates.map((c) => c.filePath))
    : new Set()
}

function toggleOne(path: string, checked: boolean) {
  const next = new Set(selectedPaths.value)
  if (checked) next.add(path)
  else next.delete(path)
  selectedPaths.value = next
}

function confirm() {
  if (selectedPaths.value.size === 0) return
  emit('confirm', {
    selectedPaths: [...selectedPaths.value],
    strategy: strategy.value,
  })
  emit('update:showing', false)
}
</script>

<template>
  <div
    v-if="showing"
    class="snap-resume-overlay"
    @click.self="closeDialog"
    @touchstart.self="closeDialog"
  >
    <div class="snap-resume-dialog" @click.stop @touchstart.stop>
      <div class="snap-resume-header">
        <p class="snap-resume-title">選擇接續掃描快照</p>
        <button type="button" class="snap-resume-close" aria-label="關閉" @click.stop="closeDialog">×</button>
      </div>

      <p v-if="loading" class="snap-resume-hint">載入快照列表…</p>
      <template v-else>
        <p class="snap-resume-hint">
          可複選多個分類快照，將依列表順序排隊接續掃描。完成度 100% 會走更新掃描；未滿 100% 會從目前完成度接續到最新。
        </p>

        <label class="snap-resume-check-all">
          <input
            type="checkbox"
            :checked="allSelected"
            :indeterminate="selectIndeterminate"
            @change="toggleAll(($event.target as HTMLInputElement).checked)"
          />
          全選
        </label>

        <ul class="snap-resume-list">
          <li v-for="c in candidates" :key="c.filePath">
            <label class="snap-resume-item">
              <input
                type="checkbox"
                :checked="selectedPaths.has(c.filePath)"
                @change="toggleOne(c.filePath, ($event.target as HTMLInputElement).checked)"
              />
              <span class="snap-resume-item-body">
                <span class="snap-resume-item-title">
                  {{ snapshotDisplayLabel(c) }} · {{ isSnapshotScanComplete(c) ? '更新' : '接續' }}
                </span>
                <span class="snap-resume-item-sub">
                  共 {{ c.totalCount }} 本 · 完成度 {{ snapshotCompletionPercent(c) }}% · 已掃
                  {{ c.scanCompletedPages }}/{{ c.totalPages }} 頁
                  <template v-if="snapshotFileDate(c)">
                    · {{ snapshotFileDate(c)!.toLocaleString() }}
                  </template>
                </span>
              </span>
            </label>
          </li>
        </ul>

        <div v-if="hasIncompleteSelection" class="snap-resume-strategy">
          <p class="snap-resume-strategy-title">接續方式</p>
          <label class="snap-resume-radio">
            <input v-model="strategy" type="radio" name="snap-strategy" value="page" />
            <span>
              <strong>頁碼接續</strong>
              <span class="snap-resume-radio-sub">從斷點與待補掃頁繼續，適合剛中斷後馬上接續。</span>
            </span>
          </label>
          <label class="snap-resume-radio">
            <input v-model="strategy" type="radio" name="snap-strategy" value="idUpdate" />
            <span>
              <strong>ID 更新式接續</strong>
              <span class="snap-resume-radio-sub">從第 1 頁掃到超過 20 個重複 ID，適合隔一段時間後接續。</span>
            </span>
          </label>
        </div>
      </template>

      <div class="snap-resume-actions">
        <button type="button" class="snap-resume-btn snap-resume-btn--ghost" @click.stop="closeDialog">
          取消
        </button>
        <button
          type="button"
          class="snap-resume-btn snap-resume-btn--ok"
          :disabled="loading || selectedPaths.size === 0"
          @click.stop="confirm"
        >
          {{
            selectedPaths.size > 0 ? `接續掃描（${selectedPaths.size} 項）` : '接續掃描'
          }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.snap-resume-overlay {
  position: fixed;
  inset: 0;
  z-index: 1300;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 12px;
  background: rgba(0, 0, 0, 0.55);
}

.snap-resume-dialog {
  width: 100%;
  max-width: 400px;
  max-height: min(88vh, 560px);
  display: flex;
  flex-direction: column;
  background: #1e1e1e;
  border: 1px solid #3c4043;
  border-radius: 12px;
  overflow: hidden;
}

.snap-resume-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px 8px;
  flex-shrink: 0;
}

.snap-resume-title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
}

.snap-resume-close {
  border: none;
  background: transparent;
  color: #9aa0a6;
  font-size: 22px;
  line-height: 1;
}

.snap-resume-hint {
  margin: 0 14px 8px;
  font-size: 11px;
  color: #9aa0a6;
  line-height: 1.4;
}

.snap-resume-check-all {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 14px 6px;
  font-size: 12px;
}

.snap-resume-list {
  list-style: none;
  margin: 0;
  padding: 0 8px;
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  border-top: 1px solid #3c4043;
  border-bottom: 1px solid #3c4043;
}

.snap-resume-item {
  display: flex;
  gap: 8px;
  padding: 8px 6px;
  font-size: 12px;
  align-items: flex-start;
}

.snap-resume-item-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  line-height: 1.35;
}

.snap-resume-item-title {
  color: #e8eaed;
}

.snap-resume-item-sub {
  font-size: 10px;
  color: #9aa0a6;
}

.snap-resume-strategy {
  padding: 10px 14px;
  flex-shrink: 0;
}

.snap-resume-strategy-title {
  margin: 0 0 6px;
  font-size: 12px;
  font-weight: 600;
}

.snap-resume-radio {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
  font-size: 11px;
  align-items: flex-start;
}

.snap-resume-radio strong {
  display: block;
  font-weight: 600;
}

.snap-resume-radio-sub {
  display: block;
  color: #9aa0a6;
  margin-top: 2px;
}

.snap-resume-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 10px 14px 12px;
  flex-shrink: 0;
}

.snap-resume-btn {
  padding: 8px 14px;
  border-radius: 6px;
  border: none;
  font-size: 12px;
}

.snap-resume-btn--ghost {
  background: #2d2d2d;
  color: #e8eaed;
}

.snap-resume-btn--ok {
  background: #3d6ef5;
  color: #fff;
}

.snap-resume-btn--ok:disabled {
  opacity: 0.45;
}
</style>
