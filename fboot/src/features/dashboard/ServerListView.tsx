import { Link } from 'react-router-dom'
import { Checkbox } from '@/components/ui/checkbox'
import { StatusBadge } from '@/components/shared/StatusBadge'
import { PowerButton } from '@/components/shared/PowerButton'
import { BootModeToggle } from '@/components/shared/BootModeToggle'
import { formatMac, formatTemp, formatWatts } from '@/lib/format'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { powerAction, updateBootConfig } from '@/store/slices/servers'
import { clearSelection, setSelection, toggleServerSelection } from '@/store/slices/ui'
import type { ServerView } from '@/hooks/useServers'

function Row({ view }: { view: ServerView }) {
  const dispatch = useAppDispatch()
  const selected = useAppSelector((s) => s.ui.selectedServerIds.includes(view.server.id))
  const { server, status, stats, bootConfig, pxeBootableName, linuxBootableName } = view
  const power = stats?.power_status ?? 'unknown'

  return (
    <tr className="border-b transition-colors hover:bg-muted/40">
      <td className="px-3 py-2">
        <Checkbox
          checked={selected}
          onCheckedChange={() => dispatch(toggleServerSelection(server.id))}
          aria-label="Select server"
        />
      </td>
      <td className="px-3 py-2">
        <Link to={`/servers/${server.id}`} className="font-medium hover:underline">
          {server.friendly_name}
        </Link>
      </td>
      <td className="px-3 py-2 font-mono text-xs text-muted-foreground">{formatMac(server.primary_mac)}</td>
      <td className="px-3 py-2">
        <StatusBadge status={power} />
      </td>
      <td className="px-3 py-2 text-sm">{status?.ip ?? '—'}</td>
      <td className="px-3 py-2">
        <BootModeToggle
          bootPxe={bootConfig?.boot_pxe ?? false}
          onChange={(pxe) => dispatch(updateBootConfig({ id: server.id, patch: { boot_pxe: pxe } }))}
        />
      </td>
      <td className="px-3 py-2 text-sm">{pxeBootableName ?? '—'}</td>
      <td className="px-3 py-2 text-sm">{linuxBootableName ?? '—'}</td>
      <td className="px-3 py-2 text-sm text-muted-foreground">{formatWatts(stats?.power_w)}</td>
      <td className="px-3 py-2 text-sm text-muted-foreground">{formatTemp(stats?.cpu_temp_c)}</td>
      <td className="px-3 py-2 text-right">
        <PowerButton
          status={power}
          onAction={(action) => dispatch(powerAction({ id: server.id, action }))}
        />
      </td>
    </tr>
  )
}

export function ServerListView({ views }: { views: ServerView[] }) {
  const dispatch = useAppDispatch()
  const selected = useAppSelector((s) => s.ui.selectedServerIds)
  const allSelected = views.length > 0 && views.every((v) => selected.includes(v.server.id))

  return (
    <div className="overflow-x-auto rounded-xl border">
      <table className="w-full text-left text-sm">
        <thead className="bg-muted/50 text-xs uppercase text-muted-foreground">
          <tr>
            <th className="px-3 py-2 font-medium">
              <Checkbox
                checked={allSelected}
                onCheckedChange={() =>
                  dispatch(allSelected ? clearSelection() : setSelection(views.map((v) => v.server.id)))
                }
                aria-label="Select all servers"
              />
            </th>
            <th className="px-3 py-2 font-medium">Name</th>
            <th className="px-3 py-2 font-medium">MAC</th>
            <th className="px-3 py-2 font-medium">Power</th>
            <th className="px-3 py-2 font-medium">IP</th>
            <th className="px-3 py-2 font-medium">Boot</th>
            <th className="px-3 py-2 font-medium">PXE Bootable</th>
            <th className="px-3 py-2 font-medium">Linux Bootable</th>
            <th className="px-3 py-2 font-medium">Watts</th>
            <th className="px-3 py-2 font-medium">Temp</th>
            <th className="px-3 py-2" />
          </tr>
        </thead>
        <tbody>
          {views.map((view) => (
            <Row key={view.server.id} view={view} />
          ))}
        </tbody>
      </table>
    </div>
  )
}
