import { useCallback, useEffect, useMemo } from 'react'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { fetchBootConfig, fetchServers } from '@/store/slices/servers'
import { fetchStats } from '@/store/slices/stats'
import { fetchBootables } from '@/store/slices/bootables'
import type { Bootable, BootConfig, Server, ServerStatus, StatsSample } from '@/api'

export interface ServerView {
  server: Server
  status?: ServerStatus
  stats?: StatsSample
  bootConfig?: BootConfig
  pxeBootableName?: string
  linuxBootableName?: string
}

function nameFor(id: string | null | undefined, bootables: Bootable[]): string | undefined {
  if (!id) return undefined
  return bootables.find((b) => b.id === id)?.name
}

export function useServerViews(): ServerView[] {
  const { byId, ids, statuses, bootConfigs } = useAppSelector((s) => s.servers)
  const stats = useAppSelector((s) => s.stats.latest)
  const bootables = useAppSelector((s) => s.bootables.items)

  return useMemo(
    () =>
      ids.map((id) => ({
        server: byId[id],
        status: statuses[id],
        stats: stats[id],
        bootConfig: bootConfigs[id],
        pxeBootableName: nameFor(bootConfigs[id]?.pxe_bootable_id, bootables),
        linuxBootableName: nameFor(bootConfigs[id]?.linux_bootable_id, bootables),
      })),
    [byId, ids, statuses, bootConfigs, stats, bootables],
  )
}

export function useServerView(id: string | undefined): ServerView | undefined {
  const views = useServerViews()
  return useMemo(() => views.find((v) => v.server?.id === id), [views, id])
}

export function useLoadServers() {
  const dispatch = useAppDispatch()
  const loading = useAppSelector((s) => s.servers.loading)
  const statsLoading = useAppSelector((s) => s.stats.loading)
  const error = useAppSelector((s) => s.servers.error)
  const ids = useAppSelector((s) => s.servers.ids)
  const bootConfigs = useAppSelector((s) => s.servers.bootConfigs)

  const reload = useCallback(() => {
    dispatch(fetchServers())
    dispatch(fetchStats())
    dispatch(fetchBootables())
  }, [dispatch])

  useEffect(() => {
    reload()
  }, [reload])

  // Boot configs aren't part of the server list payload; fetch any we're missing
  // so the dashboard boot-mode toggles reflect real state. Live changes arrive via WS.
  useEffect(() => {
    for (const id of ids) {
      if (!bootConfigs[id]) dispatch(fetchBootConfig(id))
    }
  }, [dispatch, ids, bootConfigs])

  return { loading, statsLoading, error, reload }
}
