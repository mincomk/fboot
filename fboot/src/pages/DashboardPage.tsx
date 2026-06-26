import { Server as ServerIcon } from 'lucide-react'
import { ViewToggle } from '@/components/shared/ViewToggle'
import { RefreshButton } from '@/components/shared/RefreshButton'
import { ServerGrid } from '@/features/dashboard/ServerGrid'
import { ServerListView } from '@/features/dashboard/ServerListView'
import { BatchToolbar } from '@/features/dashboard/BatchToolbar'
import { AddServerDialog } from '@/features/dashboard/AddServerDialog'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { setViewMode } from '@/store/slices/ui'
import { useLoadServers, useServerViews } from '@/hooks/useServers'

export function DashboardPage() {
  const dispatch = useAppDispatch()
  const viewMode = useAppSelector((s) => s.ui.viewMode)
  const { loading, statsLoading, error, reload } = useLoadServers()
  const views = useServerViews()

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Servers</h1>
          <p className="text-sm text-muted-foreground">{views.length} managed servers</p>
        </div>
        <div className="flex items-center gap-3">
          <RefreshButton onClick={reload} spinning={loading || statsLoading} />
          <ViewToggle value={viewMode} onChange={(mode) => dispatch(setViewMode(mode))} />
          <AddServerDialog />
        </div>
      </div>

      {error && (
        <div className="rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      {loading && views.length === 0 ? (
        <p className="text-sm text-muted-foreground">Loading servers…</p>
      ) : views.length === 0 ? (
        <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed py-16 text-center">
          <ServerIcon className="size-8 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">No servers yet.</p>
          <AddServerDialog />
        </div>
      ) : viewMode === 'card' ? (
        <ServerGrid views={views} />
      ) : (
        <ServerListView views={views} />
      )}

      <BatchToolbar />
    </div>
  )
}
