import { Checkbox } from '@/components/ui/checkbox'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { clearSelection, setSelection } from '@/store/slices/ui'
import { ServerCard } from './ServerCard'
import type { ServerView } from '@/hooks/useServers'

export function ServerGrid({ views }: { views: ServerView[] }) {
  const dispatch = useAppDispatch()
  const selected = useAppSelector((s) => s.ui.selectedServerIds)
  const allSelected = views.length > 0 && views.every((v) => selected.includes(v.server.id))

  return (
    <div className="flex flex-col gap-3">
      <label className="flex w-fit items-center gap-2 text-sm text-muted-foreground">
        <Checkbox
          checked={allSelected}
          onCheckedChange={() =>
            dispatch(allSelected ? clearSelection() : setSelection(views.map((v) => v.server.id)))
          }
          aria-label="Select all servers"
        />
        Select all
      </label>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
        {views.map((view) => (
          <ServerCard key={view.server.id} view={view} />
        ))}
      </div>
    </div>
  )
}
