export function formatWatts(w?: number | null): string {
  if (w == null) return '—'
  return `${Math.round(w)} W`
}

export function formatTemp(c?: number | null): string {
  if (c == null) return '—'
  return `${Math.round(c)}°C`
}

export function formatMac(mac?: string | null): string {
  if (!mac) return '—'
  return mac.toLowerCase()
}

export function formatRelative(ts: string | number): string {
  const then = typeof ts === 'number' ? ts : Date.parse(ts)
  if (Number.isNaN(then)) return '—'
  const diff = Date.now() - then
  const s = Math.round(diff / 1000)
  if (s < 60) return `${s}s ago`
  const m = Math.round(s / 60)
  if (m < 60) return `${m}m ago`
  const h = Math.round(m / 60)
  if (h < 24) return `${h}h ago`
  const d = Math.round(h / 24)
  return `${d}d ago`
}

export function shortId(id: string): string {
  return id.length > 8 ? id.slice(0, 8) : id
}

export function formatBytes(n?: number | null): string {
  if (n == null) return '—'
  if (n < 1024) return `${n} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let v = n / 1024
  let i = 0
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024
    i++
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`
}
