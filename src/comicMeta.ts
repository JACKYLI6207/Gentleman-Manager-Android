/** 從搜尋結果 additionalInfo 解析並顯示創建日期 */
export function formatComicCreatedLabel(additionalInfo: string): string {
  const trimmed = additionalInfo.trim()
  if (!trimmed) return ''
  const match = trimmed.match(/創建於(\d{4}-\d{2}-\d{2}(?:\s+\d{2}:\d{2}:\d{2})?)/)
  if (match?.[1]) return match[1]
  return trimmed
}
