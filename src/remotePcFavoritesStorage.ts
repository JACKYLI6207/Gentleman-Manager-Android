const STORAGE_KEY = 'gmRemotePcFavorites'

export type RemotePcFavorite = {
  name: string
  host: string
  port: number
  starredAt: number
}

function favoriteKey(host: string, port: number): string {
  return `${host.trim().toLowerCase()}:${port}`
}

export function loadRemotePcFavorites(): RemotePcFavorite[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (raw === null || raw === '') return []
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return []
    return parsed
      .filter(
        (item): item is RemotePcFavorite =>
          typeof item === 'object' &&
          item !== null &&
          typeof (item as RemotePcFavorite).name === 'string' &&
          typeof (item as RemotePcFavorite).host === 'string' &&
          typeof (item as RemotePcFavorite).port === 'number' &&
          typeof (item as RemotePcFavorite).starredAt === 'number',
      )
      .sort((a, b) => b.starredAt - a.starredAt)
  } catch {
    return []
  }
}

export function saveRemotePcFavorites(favorites: RemotePcFavorite[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(favorites))
}

export function isRemotePcFavorite(host: string, port: number, favorites: RemotePcFavorite[]): boolean {
  const key = favoriteKey(host, port)
  return favorites.some((f) => favoriteKey(f.host, f.port) === key)
}

export function toggleRemotePcFavorite(
  name: string,
  host: string,
  port: number,
): RemotePcFavorite[] {
  const favorites = loadRemotePcFavorites()
  const key = favoriteKey(host, port)
  const index = favorites.findIndex((f) => favoriteKey(f.host, f.port) === key)
  if (index >= 0) {
    favorites.splice(index, 1)
  } else {
    favorites.unshift({
      name: name.trim() || `PC (${host})`,
      host: host.trim(),
      port,
      starredAt: Date.now(),
    })
  }
  saveRemotePcFavorites(favorites)
  return favorites
}

export function removeRemotePcFavorite(host: string, port: number): RemotePcFavorite[] {
  const key = favoriteKey(host, port)
  const favorites = loadRemotePcFavorites().filter((f) => favoriteKey(f.host, f.port) !== key)
  saveRemotePcFavorites(favorites)
  return favorites
}
