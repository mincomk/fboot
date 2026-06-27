import { useState } from 'react'
import { ChevronRight, Database, RefreshCw, Trash2 } from 'lucide-react'
import { api } from '@/api'
import type { CacheEntry, CacheNamespace } from '@/api'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { cn } from '@/lib/utils'
import { formatRelative } from '@/lib/format'

// Target of a pending clear confirmation: a single namespace, or all of them.
type ClearTarget = { kind: 'all' } | { kind: 'namespace'; namespace: string }

function previewValue(value: string): string {
  let text = value
  try {
    text = JSON.stringify(JSON.parse(value))
  } catch {
    /* not json — show raw */
  }
  return text.length > 160 ? `${text.slice(0, 160)}…` : text
}

export function CacheSection() {
  const [namespaces, setNamespaces] = useState<CacheNamespace[] | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [expanded, setExpanded] = useState<string | null>(null)
  const [entries, setEntries] = useState<Record<string, CacheEntry[]>>({})
  const [entriesLoading, setEntriesLoading] = useState<string | null>(null)
  const [entriesError, setEntriesError] = useState<string | null>(null)

  const [clearTarget, setClearTarget] = useState<ClearTarget | null>(null)
  const [clearing, setClearing] = useState(false)

  const loadNamespaces = async () => {
    setLoading(true)
    setError(null)
    try {
      const view = await api.cache.view()
      setNamespaces(view)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load cache')
    } finally {
      setLoading(false)
    }
  }

  const toggleNamespace = async (namespace: string) => {
    if (expanded === namespace) {
      setExpanded(null)
      return
    }
    setExpanded(namespace)
    setEntriesError(null)
    if (entries[namespace]) return
    setEntriesLoading(namespace)
    try {
      const rows = await api.cache.entries(namespace)
      setEntries((prev) => ({ ...prev, [namespace]: rows }))
    } catch (e) {
      setEntriesError(e instanceof Error ? e.message : 'Failed to load entries')
    } finally {
      setEntriesLoading(null)
    }
  }

  const confirmClear = async () => {
    if (!clearTarget) return
    setClearing(true)
    setError(null)
    try {
      await api.cache.clear(clearTarget.kind === 'namespace' ? clearTarget.namespace : undefined)
      setEntries({})
      setExpanded(null)
      setClearTarget(null)
      await loadNamespaces()
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to clear cache')
    } finally {
      setClearing(false)
    }
  }

  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between gap-4">
        <div className="space-y-1.5">
          <CardTitle className="flex items-center gap-2">
            <Database className="size-4 text-muted-foreground" /> Cache
          </CardTitle>
          <CardDescription>Inspect and clear cached temporal state (e.g. ARP).</CardDescription>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button variant="outline" size="sm" onClick={loadNamespaces} disabled={loading}>
            <RefreshCw className={cn('size-4', loading && 'animate-spin')} />
            {namespaces ? 'Refresh' : 'View cache'}
          </Button>
          <Button
            variant="destructive"
            size="sm"
            onClick={() => setClearTarget({ kind: 'all' })}
            disabled={loading || !namespaces?.length}
          >
            <Trash2 className="size-4" /> Clear all
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {error && <p className="mb-3 text-sm text-destructive">{error}</p>}

        {namespaces === null ? (
          <p className="text-sm text-muted-foreground">
            Load the cache to inspect namespaces and their entries.
          </p>
        ) : namespaces.length === 0 ? (
          <p className="text-sm text-muted-foreground">The cache is empty.</p>
        ) : (
          <div className="divide-y rounded-md border">
            {namespaces.map((ns) => {
              const isOpen = expanded === ns.namespace
              const rows = entries[ns.namespace]
              return (
                <div key={ns.namespace}>
                  <div className="flex items-center gap-3 px-3 py-2.5">
                    <button
                      type="button"
                      onClick={() => toggleNamespace(ns.namespace)}
                      className="flex min-w-0 flex-1 items-center gap-2 text-left"
                    >
                      <ChevronRight
                        className={cn(
                          'size-4 shrink-0 text-muted-foreground transition-transform',
                          isOpen && 'rotate-90',
                        )}
                      />
                      <span className="truncate font-mono text-sm">{ns.namespace}</span>
                      <Badge variant="secondary">{ns.count}</Badge>
                    </button>
                    <div className="hidden shrink-0 text-xs text-muted-foreground sm:block">
                      {ns.oldest && ns.newest ? (
                        <span>
                          {formatRelative(ns.oldest)} → {formatRelative(ns.newest)}
                        </span>
                      ) : (
                        '—'
                      )}
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="size-8 shrink-0 text-muted-foreground hover:text-destructive"
                      onClick={() => setClearTarget({ kind: 'namespace', namespace: ns.namespace })}
                      aria-label={`Clear ${ns.namespace}`}
                    >
                      <Trash2 className="size-4" />
                    </Button>
                  </div>
                  {isOpen && (
                    <div className="bg-muted/30 px-3 py-2">
                      {entriesLoading === ns.namespace ? (
                        <p className="text-xs text-muted-foreground">Loading entries…</p>
                      ) : entriesError ? (
                        <p className="text-xs text-destructive">{entriesError}</p>
                      ) : rows && rows.length > 0 ? (
                        <div className="flex flex-col gap-1.5">
                          {rows.map((entry) => (
                            <div
                              key={entry.key}
                              className="flex flex-col gap-0.5 rounded border bg-background px-2 py-1.5"
                            >
                              <div className="flex items-center justify-between gap-2">
                                <span className="truncate font-mono text-xs">{entry.key}</span>
                                <span className="shrink-0 text-[11px] text-muted-foreground">
                                  {formatRelative(entry.updated_at)}
                                </span>
                              </div>
                              <span className="truncate font-mono text-[11px] text-muted-foreground">
                                {previewValue(entry.value)}
                              </span>
                            </div>
                          ))}
                        </div>
                      ) : (
                        <p className="text-xs text-muted-foreground">No entries.</p>
                      )}
                    </div>
                  )}
                </div>
              )
            })}
          </div>
        )}
      </CardContent>

      <Dialog open={clearTarget !== null} onOpenChange={(open) => !open && setClearTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {clearTarget?.kind === 'namespace'
                ? `Clear "${clearTarget.namespace}"?`
                : 'Clear all cache?'}
            </DialogTitle>
            <DialogDescription>
              This removes{' '}
              {clearTarget?.kind === 'namespace' ? 'this namespace of ' : 'all '}
              cached temporal state, e.g. ARP. It will be rebuilt automatically.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setClearTarget(null)} disabled={clearing}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={confirmClear} disabled={clearing}>
              {clearing ? 'Clearing…' : 'Clear'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  )
}
