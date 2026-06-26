import { Link } from 'react-router-dom'
import { Cpu, Network, Zap } from 'lucide-react'
import { Card, CardContent, CardHeader } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { StatusBadge } from '@/components/shared/StatusBadge'
import { PowerButton } from '@/components/shared/PowerButton'
import { BootModeToggle } from '@/components/shared/BootModeToggle'
import { ReachIndicator } from '@/components/shared/ReachIndicator'
import { formatMac, formatTemp, formatWatts } from '@/lib/format'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { powerAction, updateBootConfig } from '@/store/slices/servers'
import { toggleServerSelection } from '@/store/slices/ui'
import type { ServerView } from '@/hooks/useServers'

export function ServerCard({ view }: { view: ServerView }) {
  const dispatch = useAppDispatch()
  const selected = useAppSelector((s) => s.ui.selectedServerIds.includes(view.server.id))
  const { server, status, stats, bootConfig, pxeBootableName, linuxBootableName } = view
  const power = stats?.power_status ?? 'unknown'

  return (
    <Card className="relative transition-shadow hover:shadow-md">
      <CardHeader className="flex-row items-start justify-between gap-2">
        <div className="flex items-start gap-3">
          <Checkbox
            checked={selected}
            onCheckedChange={() => dispatch(toggleServerSelection(server.id))}
            aria-label="Select server"
            className="mt-1"
          />
          <div>
            <Link to={`/servers/${server.id}`} className="font-semibold hover:underline">
              {server.friendly_name}
            </Link>
            <p className="font-mono text-xs text-muted-foreground">{formatMac(server.primary_mac)}</p>
          </div>
        </div>
        <StatusBadge status={power} />
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5">
          <ReachIndicator label="IPMI" active={status?.ipmi_reachable ?? false} />
          <span className="text-xs text-muted-foreground">
            {status?.ip ? status.ip : 'No IP'}
          </span>
          <BootModeToggle
            bootPxe={bootConfig?.boot_pxe ?? false}
            onChange={(pxe) => dispatch(updateBootConfig({ id: server.id, patch: { boot_pxe: pxe } }))}
          />
        </div>

        <div className="flex flex-col gap-0.5 text-xs text-muted-foreground">
          <p className="truncate">
            PXE: <span className="font-medium text-foreground">{pxeBootableName ?? '—'}</span>
          </p>
          <p className="truncate">
            Linux: <span className="font-medium text-foreground">{linuxBootableName ?? '—'}</span>
          </p>
        </div>

        <div className="flex items-center gap-4 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1">
            <Zap className="size-3.5" /> {formatWatts(stats?.power_w)}
          </span>
          <span className="inline-flex items-center gap-1">
            <Cpu className="size-3.5" /> {formatTemp(stats?.cpu_temp_c)}
          </span>
          <span className="inline-flex items-center gap-1">
            <Network className="size-3.5" /> {status?.online ? 'Online' : 'Offline'}
          </span>
        </div>

        <div className="pt-1">
          <PowerButton
            status={power}
            onAction={(action) => dispatch(powerAction({ id: server.id, action }))}
          />
        </div>
      </CardContent>
    </Card>
  )
}
