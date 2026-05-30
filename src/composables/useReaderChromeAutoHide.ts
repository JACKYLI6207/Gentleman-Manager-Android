import { onBeforeUnmount, ref, watch } from 'vue'
import { readerFullscreenActive } from './useReaderFullscreen'

const IDLE_MS = 1000

/** 全視窗閱讀：捲動時隱藏底部控制列，靜止超過 1 秒後再彈出 */
export function useReaderChromeAutoHide() {
  const chromeVisible = ref(true)
  let idleTimer: ReturnType<typeof setTimeout> | undefined

  function clearIdleTimer() {
    if (idleTimer !== undefined) {
      clearTimeout(idleTimer)
      idleTimer = undefined
    }
  }

  function scheduleShowAfterIdle() {
    clearIdleTimer()
    idleTimer = setTimeout(() => {
      chromeVisible.value = true
      idleTimer = undefined
    }, IDLE_MS)
  }

  function onReaderScrollForChrome() {
    if (!readerFullscreenActive.value) {
      if (!chromeVisible.value) chromeVisible.value = true
      clearIdleTimer()
      return
    }
    chromeVisible.value = false
    scheduleShowAfterIdle()
  }

  watch(readerFullscreenActive, (fs) => {
    clearIdleTimer()
    chromeVisible.value = true
    if (!fs) chromeVisible.value = true
  })

  onBeforeUnmount(() => {
    clearIdleTimer()
  })

  return { chromeVisible, onReaderScrollForChrome }
}
