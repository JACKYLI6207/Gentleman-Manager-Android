/** 與 Rust `config.rs` 的 `DEFAULT_API_DOMAIN` 一致 */
export const DEFAULT_API_DOMAIN = 'www.wn07.ru'

/** 設定「自定義」時可選的 API 域名（順序固定） */
export const API_DOMAIN_OPTIONS: readonly string[] = [
  'www.wn01.cfd',
  'www.wn02.cfd',
  'www.wn03.cfd',
  'www.wn04.cfd',
  'www.wn05.cfd',
  'www.wn06.cfd',
  'www.wn07.cfd',
  'www.wn08.cfd',
  'www.wn09.cfd',
  'www.wn10.cfd',
  'www.wn01.shop',
  'www.wn02.shop',
  'www.wn03.shop',
  'www.wn04.shop',
  'www.wn05.shop',
  'www.wn06.shop',
  'www.wn07.shop',
  'www.wn08.shop',
  'www.wn09.shop',
  'www.wn10.shop',
  'www.wn01.ru',
  'www.wn02.ru',
  'www.wn03.ru',
  'www.wn04.ru',
  'www.wn05.ru',
  'www.wn06.ru',
  'www.wn07.ru',
  'www.wn08.ru',
  'www.wn09.ru',
  'www.wn10.ru',
  'www.wnacg.com',
] as const

export type ApiDomainMode = 'Default' | 'Custom'

export function getActiveApiDomain(mode: ApiDomainMode, customApiDomain: string): string {
  if (mode === 'Default') return DEFAULT_API_DOMAIN
  const trimmed = customApiDomain.trim()
  return trimmed || DEFAULT_API_DOMAIN
}
