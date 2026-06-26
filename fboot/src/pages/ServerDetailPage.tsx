import { useEffect } from 'react'
import { Link, useParams } from 'react-router-dom'
import { ArrowLeft, Info, HardDrive, KeyRound, TerminalSquare, Power } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { RefreshButton } from '@/components/shared/RefreshButton'
import { PowerButton } from '@/components/shared/PowerButton'
import { DetailLayout, type DetailSection } from '@/features/server-detail/DetailLayout'
import { InfoSection } from '@/features/server-detail/sections/InfoSection'
import { BootManagementSection } from '@/features/server-detail/sections/BootManagementSection'
import { IpmiSection } from '@/features/server-detail/sections/IpmiSection'
import { TerminalSection } from '@/features/server-detail/sections/TerminalSection'
import { PowerSection } from '@/features/server-detail/sections/PowerSection'
import { useLoadServers, useServerView } from '@/hooks/useServers'
import { useRecentServers } from '@/hooks/useRecentServers'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { fetchBootConfig, fetchServers, powerAction } from '@/store/slices/servers'
import { fetchStats } from '@/store/slices/stats'

const SECTIONS: DetailSection[] = [
  { key: 'info', label: 'Server Info', icon: <Info className="size-4" /> },
  { key: 'boot', label: 'Boot Management', icon: <HardDrive className="size-4" /> },
  { key: 'ipmi', label: 'IPMI', icon: <KeyRound className="size-4" /> },
  { key: 'terminal', label: 'Terminal', icon: <TerminalSquare className="size-4" /> },
  { key: 'power', label: 'On / Off', icon: <Power className="size-4" /> },
]

const SECTION_KEYS = SECTIONS.map((s) => s.key)

export function ServerDetailPage() {
  const { id, section } = useParams<{ id: string; section: string }>()
  const dispatch = useAppDispatch()
  const active = section && SECTION_KEYS.includes(section) ? section : 'info'
  const { recent, visit } = useRecentServers()
  const { loading, statsLoading } = useLoadServers()
  const view = useServerView(id)
  const power = useAppSelector((s) =>
    id ? (s.stats.latest[id]?.power_status ?? 'unknown') : 'unknown',
  )

  useEffect(() => {
    if (id) visit(id)
  }, [id, visit])

  const refresh = () => {
    dispatch(fetchServers())
    dispatch(fetchStats())
    if (id) dispatch(fetchBootConfig(id))
  }

  if (!id) return null

  if (!view) {
    return (
      <div className="flex flex-col gap-4">
        <Button asChild variant="ghost" size="sm" className="w-fit">
          <Link to="/">
            <ArrowLeft /> Back
          </Link>
        </Button>
        <p className="text-sm text-muted-foreground">Server not found.</p>
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <Button asChild variant="ghost" size="sm" className="w-fit">
          <Link to="/">
            <ArrowLeft /> Dashboard
          </Link>
        </Button>
        <RefreshButton onClick={refresh} spinning={loading || statsLoading} />
      </div>

      <DetailLayout
        title={view.server.friendly_name}
        subtitle={view.server.primary_mac}
        recent={recent}
        activeId={id}
        sections={SECTIONS}
        powerSlot={
          <PowerButton
            status={power}
            onAction={(action) => dispatch(powerAction({ id, action }))}
          />
        }
      >
        {active === 'info' && <InfoSection view={view} />}
        {active === 'boot' && <BootManagementSection view={view} />}
        {active === 'ipmi' && <IpmiSection view={view} />}
        {active === 'terminal' && <TerminalSection view={view} />}
        {active === 'power' && <PowerSection view={view} />}
      </DetailLayout>
    </div>
  )
}
