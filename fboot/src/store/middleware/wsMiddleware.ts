import type { Middleware } from '@reduxjs/toolkit'
import { ws } from '@/api'
import type { EventConnection, ServerEvent } from '@/api'
import {
  bootConfigChanged,
  serverRemoved,
  serverUpserted,
  statusChanged,
} from '@/store/slices/servers'
import { statsUpdated } from '@/store/slices/stats'
import { consoleStatusChanged } from '@/store/slices/console'
import { setWsStatus } from '@/store/slices/ui'

export const wsConnect = { type: 'ws/connect' as const }
export const wsDisconnect = { type: 'ws/disconnect' as const }

export const wsMiddleware: Middleware = (store) => {
  let connection: EventConnection | null = null

  const dispatchEvent = (event: ServerEvent) => {
    switch (event.type) {
      case 'server_added':
      case 'server_updated':
        store.dispatch(serverUpserted(event.server))
        break
      case 'server_removed':
        store.dispatch(serverRemoved(event.id))
        break
      case 'status_changed':
        store.dispatch(statusChanged(event.status))
        break
      case 'stats_updated':
        store.dispatch(statsUpdated(event.sample))
        break
      case 'boot_config_changed':
        store.dispatch(bootConfigChanged(event.config))
        break
      case 'console_status_changed':
        store.dispatch(
          consoleStatusChanged({ server_id: event.server_id, status: event.status }),
        )
        break
    }
  }

  return (next) => (action) => {
    const type =
      typeof action === 'object' && action && 'type' in action ? (action.type as string) : ''
    if (type === wsConnect.type && !connection) {
      store.dispatch(setWsStatus('connecting'))
      connection = ws.connectEvents({
        onEvent: dispatchEvent,
        onOpen: () => store.dispatch(setWsStatus('open')),
        onClose: () => store.dispatch(setWsStatus('closed')),
      })
    } else if (type === wsDisconnect.type) {
      connection?.close()
      connection = null
      store.dispatch(setWsStatus('closed'))
    }
    return next(action)
  }
}
