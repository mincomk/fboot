import { Link } from 'react-router-dom'
import { History } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useAppSelector } from '@/store/hooks'

export interface RecentServersSidebarProps {
  recent: string[]
  activeId: string
}

export function RecentServersSidebar({ recent, activeId }: RecentServersSidebarProps) {
  const byId = useAppSelector((s) => s.servers.byId)
  const entries = recent.map((id) => byId[id]).filter(Boolean)

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-2 px-2 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        <History className="size-3.5" /> Recently used
      </div>
      {entries.length === 0 ? (
        <p className="px-2 text-xs text-muted-foreground">No recent servers.</p>
      ) : (
        entries.map((server) => (
          <Link
            key={server.id}
            to={`/servers/${server.id}`}
            className={cn(
              'truncate rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-accent',
              server.id === activeId && 'bg-accent font-medium',
            )}
          >
            {server.friendly_name}
          </Link>
        ))
      )}
    </div>
  )
}
