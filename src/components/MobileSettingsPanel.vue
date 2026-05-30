<script setup lang="ts">

import { computed, onMounted, ref } from 'vue'

import type { Config } from '../api'

import { getConfig, pickDownloadDirectory, pickKoreanTxtFile, saveConfig } from '../api'

import { getActiveApiDomain } from '../apiDomains'

import ApiDomainListDialog from './ApiDomainListDialog.vue'



const props = defineProps<{

  categoryDir: string

  categoryCount?: number

}>()



const emit = defineEmits<{

  pickCategory: []

}>()



const config = ref<Config | null>(null)

const saving = ref(false)

const status = ref('')

const apiDomainListOpen = ref(false)

const activeApiDomain = computed(() => {
  if (!config.value) return ''
  return getActiveApiDomain(config.value.apiDomainMode, config.value.customApiDomain)
})



async function load() {

  try {

    config.value = await getConfig()

  } catch (e) {

    status.value = String(e)

  }

}



async function persist() {

  if (!config.value) return

  saving.value = true

  try {

    await saveConfig(config.value)

    status.value = '已儲存'

  } catch (e) {

    status.value = String(e)

  } finally {

    saving.value = false

  }

}



async function pickDownload() {

  const path = await pickDownloadDirectory()

  if (path && config.value) {

    config.value.downloadDir = path

    await persist()

  }

}



async function pickKoreanTxt() {

  const path = await pickKoreanTxtFile()

  if (path && config.value) {

    config.value.koreanTxtCatalogDir = path

    await persist()

  }

}

async function selectApiDomainDefault() {

  if (!config.value) return

  config.value.apiDomainMode = 'Default'

  await persist()

}

function openApiDomainList() {

  apiDomainListOpen.value = true

}

async function onApiDomainPicked(domain: string) {

  if (!config.value) return

  config.value.apiDomainMode = 'Custom'

  config.value.customApiDomain = domain

  await persist()

}



function dirHint(path: string, emptyLabel: string) {

  const p = path.trim()

  return p || emptyLabel

}

function txtFileHint(path: string, emptyLabel: string) {

  const p = path.trim()

  if (!p) return emptyLabel

  const tail = p.split('/').pop() ?? p

  try {

    return decodeURIComponent(tail)

  } catch {

    return tail

  }

}



onMounted(() => void load())

</script>



<template>

  <div v-if="config" class="settings">

    <h3>API域名</h3>

    <div class="api-domain-segment" role="group" aria-label="API域名模式">

      <button

        type="button"

        class="api-domain-segment-btn"

        :class="{ 'api-domain-segment-btn--on': config.apiDomainMode === 'Default' }"

        @click="selectApiDomainDefault"

      >

        默認

      </button>

      <button

        type="button"

        class="api-domain-segment-btn"

        :class="{ 'api-domain-segment-btn--on': config.apiDomainMode === 'Custom' }"

        @click="openApiDomainList"

      >

        自定義

      </button>

    </div>

    <p class="api-domain-current">

      目前：<span class="api-domain-current-value">{{ activeApiDomain }}</span>

    </p>



    <h3>下載格式</h3>

    <label class="radio-row">

      <input v-model="config.downloadFormat" type="radio" value="Server2Zip" @change="persist" />

      透過 SERVER2 下載 ZIP

    </label>

    <label class="radio-row">

      <input v-model="config.downloadFormat" type="radio" value="JpegZipPack" @change="persist" />

      JPEG 逐張打包 ZIP

    </label>



    <h3>下載失敗重試</h3>

    <p class="hint">最多再嘗試 {{ config.downloadRetryCount }} 次（總共 {{ config.downloadRetryCount + 1 }} 次）</p>

    <input v-model.number="config.downloadRetryCount" type="range" min="0" max="20" @change="persist" />



    <h3>下載失敗休息</h3>

    <p class="hint">
      任務進入「下載失敗」時暫停佇列 {{ config.downloadFailureRestSec }} 秒後自動繼續（0 表示關閉）
    </p>

    <input v-model.number="config.downloadFailureRestSec" type="range" min="0" max="600" @change="persist" />



    <h3>下載速度</h3>

    <p class="hint">同一時間僅一本漫畫實際下載；休息時間在該本完成後、下一本開始前生效</p>

    <p class="hint">每本漫畫下載完成後休息 {{ config.comicDownloadIntervalSec }} 秒</p>

    <input v-model.number="config.comicDownloadIntervalSec" type="range" min="0" max="120" @change="persist" />



    <h3>韓漫 TXT 重複檢查</h3>

    <label class="check-row">

      <input v-model="config.koreanTxtDuplicateCheckEnabled" type="checkbox" @change="persist" />

      韓漫下載時比對 TXT 列表

    </label>

    <div class="dir-block">

      <button type="button" class="dir-btn" @click="pickKoreanTxt">選擇 TXT 檔案</button>

      <p class="dir-path">{{ txtFileHint(config.koreanTxtCatalogDir, '尚未選擇 TXT 檔案') }}</p>

    </div>



    <h3>目錄</h3>

    <div class="dir-block">

      <button type="button" class="dir-btn" @click="pickDownload">指定下載目錄</button>

      <p class="dir-path">{{ dirHint(config.downloadDir, '尚未指定下載目錄') }}</p>

    </div>

    <div class="dir-block">

      <button type="button" class="dir-btn" @click="emit('pickCategory')">

        讀取分類目錄（快照）

      </button>

      <p class="dir-path">

        <template v-if="categoryDir.trim()">

          {{ categoryDir.trim() }}

          <span v-if="(categoryCount ?? 0) > 0" class="dir-meta"> · 已載入 {{ categoryCount }} 個分類</span>

        </template>

        <template v-else>尚未選擇快照目錄</template>

      </p>

    </div>



    <h3>快照更新</h3>

    <p class="hint">ID 更新式掃描：舊 ID 重複超過此數後，再連續 2 頁無新增才停止</p>

    <label class="num-row">

      <span>快照更新 ID 重複停止數</span>

      <input

        v-model.number="config.snapshotUpdateDuplicateStopCount"

        type="number"

        min="0"

        max="9999"

        step="1"

        @change="persist"

      />

    </label>



    <p v-if="status" class="status">{{ status }}</p>

    <p v-if="saving" class="hint">儲存中…</p>

  </div>

  <p v-else class="hint">載入設定中…</p>



  <ApiDomainListDialog

    v-model:showing="apiDomainListOpen"

    :selected-domain="activeApiDomain"

    @select="onApiDomainPicked"

  />

</template>



<style scoped>

.settings {

  padding: 10px 12px 24px;

  overflow-y: auto;

}

h3 {

  margin: 14px 0 6px;

  font-size: 13px;

  font-weight: 700;

}

.api-domain-segment {

  display: flex;

  border-radius: 8px;

  overflow: hidden;

  border: 1px solid #5f6368;

  max-width: 280px;

}

.api-domain-segment-btn {

  flex: 1;

  padding: 8px 12px;

  border: none;

  background: #2d2d2d;

  color: #e8eaed;

  font-size: 12px;

}

.api-domain-segment-btn--on {

  background: #3d6ef5;

  color: #fff;

}

.api-domain-current {

  margin: 8px 0 4px;

  font-size: 11px;

  color: #9aa0a6;

  word-break: break-all;

}

.api-domain-current-value {

  color: #8ab4f8;

}

.radio-row,

.check-row,

.num-row {

  display: flex;

  align-items: center;

  gap: 8px;

  font-size: 12px;

  margin-bottom: 6px;

}

.num-row input[type='number'] {

  width: 72px;

  margin-left: auto;

  padding: 4px 6px;

  border-radius: 4px;

  border: 1px solid #5f6368;

  background: #1e1e1e;

  color: #e8eaed;

  font-size: 12px;

}

.dir-block {

  margin-bottom: 12px;

}

.dir-btn {

  display: inline-block;

  padding: 8px 14px;

  border: none;

  border-radius: 6px;

  background: #3d6ef5;

  color: #fff;

  font-size: 12px;

  max-width: 100%;

}

.dir-path {

  margin: 6px 0 0;

  font-size: 11px;

  color: #8ab4f8;

  word-break: break-all;

  line-height: 1.35;

}

.dir-meta {

  color: #9aa0a6;

}

.hint {

  font-size: 11px;

  color: #999;

  margin: 4px 0;

  word-break: break-all;

}

.status {

  margin-top: 10px;

  font-size: 11px;

  color: #8fd18f;

}

input[type='range'] {

  width: 100%;

}

</style>


