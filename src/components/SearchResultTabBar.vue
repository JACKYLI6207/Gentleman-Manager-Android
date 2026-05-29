<script setup lang="ts">
import type { MobileSearchTab } from '../searchResultTabs'

defineProps<{
  tabs: MobileSearchTab[]
  activeId: string | null
  isBookmarked: (tabId: string) => boolean
}>()

const emit = defineEmits<{
  select: [id: string]
  close: [id: string]
  toggleBookmark: [id: string]
}>()
</script>

<template>
  <div v-if="tabs.length > 0" class="result-tab-bar" role="tablist">
    <div
      v-for="tab in tabs"
      :key="tab.id"
      class="result-tab"
      :class="{ on: tab.id === activeId }"
      role="tab"
      :aria-selected="tab.id === activeId"
      :title="tab.title"
      @click="emit('select', tab.id)"
    >
      <button
        type="button"
        class="tab-star"
        :class="{ on: isBookmarked(tab.id) }"
        :title="isBookmarked(tab.id) ? '從收藏分頁移除' : '加入收藏分頁'"
        @click.stop="emit('toggleBookmark', tab.id)"
      >
        {{ isBookmarked(tab.id) ? '★' : '☆' }}
      </button>
      <span class="tab-title">{{ tab.title }}</span>
      <button type="button" class="tab-close" title="關閉分頁" @click.stop="emit('close', tab.id)">×</button>
    </div>
  </div>
</template>

<style scoped>
.result-tab-bar {
  display: flex;
  align-items: stretch;
  gap: 2px;
  width: 100%;
  min-width: 0;
  padding: 0 6px;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
  min-height: 30px;
  border-bottom: 1px solid #2a2a2a;
  background: #141414;
}

.result-tab {
  display: flex;
  align-items: center;
  gap: 4px;
  flex: 1 1 0;
  min-width: 4.5rem;
  max-width: 11rem;
  padding: 4px 6px 4px 8px;
  border: 1px solid transparent;
  border-bottom: none;
  border-radius: 4px 4px 0 0;
  background: transparent;
  color: #bbb;
  cursor: pointer;
  overflow: hidden;
  font-size: 12px;
  line-height: 1.2;
}

.result-tab.on {
  color: #fff;
  font-weight: 600;
  background: rgba(255, 255, 255, 0.08);
  border-color: #444;
}

.tab-star {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  border-radius: 2px;
  background: transparent;
  color: inherit;
  font-size: 12px;
  line-height: 1;
  opacity: 0.55;
}

.tab-star.on {
  opacity: 1;
  color: #f0c040;
}

.tab-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tab-close {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  padding: 0;
  border: none;
  border-radius: 2px;
  background: transparent;
  color: inherit;
  font-size: 14px;
  line-height: 1;
  opacity: 0.65;
}
</style>
