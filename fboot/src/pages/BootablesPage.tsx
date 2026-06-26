import { useEffect } from 'react'
import { HardDriveDownload } from 'lucide-react'
import { NewBootableDialog } from '@/features/bootables/NewBootableDialog'
import { RefreshButton } from '@/components/shared/RefreshButton'
import { BootableCard } from '@/features/bootables/BootableCard'
import { DefaultBootCard } from '@/features/bootables/DefaultBootCard'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { fetchBootables } from '@/store/slices/bootables'

export function BootablesPage() {
  const dispatch = useAppDispatch()
  const { items, loading, error } = useAppSelector((s) => s.bootables)

  useEffect(() => {
    dispatch(fetchBootables())
  }, [dispatch])

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Bootables</h1>
          <p className="text-sm text-muted-foreground">{items.length} bootable images</p>
        </div>
        <div className="flex items-center gap-3">
          <RefreshButton onClick={() => dispatch(fetchBootables())} spinning={loading} />
          <NewBootableDialog />
        </div>
      </div>

      {error && (
        <div className="rounded-md border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error}
        </div>
      )}

      <DefaultBootCard />

      {loading && items.length === 0 ? (
        <p className="text-sm text-muted-foreground">Loading bootables…</p>
      ) : items.length === 0 ? (
        <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed py-16 text-center">
          <HardDriveDownload className="size-8 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">No bootables yet.</p>
          <NewBootableDialog />
        </div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {items.map((b) => (
            <BootableCard key={b.id} bootable={b} />
          ))}
        </div>
      )}
    </div>
  )
}
