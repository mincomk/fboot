import type { ScanEvent, ScanOptions, ServerEvent } from './types'

type GetToken = () => string | null | undefined

export interface WsClientOptions {
  getToken?: GetToken
}

function resolveWsUrl(path: string, getToken?: GetToken, wsBase?: string): string {
  let base: string
  if (wsBase) {
    const origin = wsBase.replace(/^http/, 'ws').replace(/\/$/, '')
    base = `${origin}${path}`
  } else {
    const proto = window.location.protocol === 'https:' ? 'wss' : 'ws'
    base = `${proto}://${window.location.host}${path}`
  }
  const token = getToken?.()
  if (!token) return base
  const sep = path.includes('?') ? '&' : '?'
  return `${base}${sep}access_token=${encodeURIComponent(token)}`
}

export interface EventConnection {
  close: () => void
}

export interface EventHandlers {
  onEvent: (event: ServerEvent) => void
  onOpen?: () => void
  onClose?: () => void
  onError?: (err: Event) => void
}

export interface ConsoleHandlers {
  onData: (data: string) => void
  onOpen?: () => void
  onClose?: () => void
  onError?: (err: Event) => void
}

export interface ConsoleConnection {
  send: (data: string) => void
  close: () => void
}

const HEARTBEAT_TIMEOUT = 45000

export function createWsClient(getToken?: GetToken, wsBase?: string) {
  function connectEvents(handlers: EventHandlers): EventConnection {
    let socket: WebSocket | null = null
    let closed = false
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null
    let heartbeatTimer: ReturnType<typeof setTimeout> | null = null

    const bumpHeartbeat = () => {
      if (heartbeatTimer) clearTimeout(heartbeatTimer)
      heartbeatTimer = setTimeout(() => socket?.close(), HEARTBEAT_TIMEOUT)
    }

    const open = () => {
      if (closed) return
      socket = new WebSocket(resolveWsUrl('/ws', getToken, wsBase))
      socket.onopen = () => {
        bumpHeartbeat()
        handlers.onOpen?.()
      }
      socket.onmessage = (msg) => {
        bumpHeartbeat()
        let text: string
        if (typeof msg.data === 'string') text = msg.data
        else return
        if (text === 'ping' || text === 'pong' || text === 'heartbeat') {
          socket?.send('pong')
          return
        }
        try {
          const parsed = JSON.parse(text)
          if (parsed && (parsed.type === 'ping' || parsed.type === 'heartbeat')) {
            socket?.send(JSON.stringify({ type: 'pong' }))
            return
          }
          handlers.onEvent(parsed as ServerEvent)
        } catch {
          /* ignore malformed frame */
        }
      }
      socket.onerror = (err) => handlers.onError?.(err)
      socket.onclose = () => {
        handlers.onClose?.()
        if (heartbeatTimer) clearTimeout(heartbeatTimer)
        if (!closed) reconnectTimer = setTimeout(open, 2000)
      }
    }

    open()

    return {
      close: () => {
        closed = true
        if (reconnectTimer) clearTimeout(reconnectTimer)
        if (heartbeatTimer) clearTimeout(heartbeatTimer)
        socket?.close()
      },
    }
  }

  function connectConsole(serverId: string, handlers: ConsoleHandlers): ConsoleConnection {
    const socket = new WebSocket(resolveWsUrl(`/ws/console/${serverId}`, getToken, wsBase))
    socket.binaryType = 'arraybuffer'
    const decoder = new TextDecoder()
    socket.onopen = () => handlers.onOpen?.()
    socket.onmessage = (msg) => {
      if (typeof msg.data === 'string') handlers.onData(msg.data)
      else handlers.onData(decoder.decode(new Uint8Array(msg.data)))
    }
    socket.onerror = (err) => handlers.onError?.(err)
    socket.onclose = () => handlers.onClose?.()

    return {
      send: (data: string) => {
        if (socket.readyState === WebSocket.OPEN) socket.send(data)
      },
      close: () => socket.close(),
    }
  }

  function connectScan(
    options: ScanOptions,
    onEvent: (event: ScanEvent) => void,
    onClose?: () => void,
  ): EventConnection {
    const params = new URLSearchParams()
    if (options.cidr) params.set('cidr', options.cidr)
    if (options.probe_ipmi != null) params.set('probe_ipmi', String(options.probe_ipmi))
    if (options.probe_ssh != null) params.set('probe_ssh', String(options.probe_ssh))
    if (options.ports?.length) params.set('ports', options.ports.join(','))
    const query = params.toString()
    const socket = new WebSocket(resolveWsUrl(`/api/scan/ws${query ? `?${query}` : ''}`, getToken, wsBase))
    socket.onmessage = (msg) => {
      if (typeof msg.data !== 'string') return
      try {
        onEvent(JSON.parse(msg.data) as ScanEvent)
      } catch {
        /* ignore */
      }
    }
    socket.onclose = () => onClose?.()
    return { close: () => socket.close() }
  }

  return { connectEvents, connectConsole, connectScan }
}

export type WsClient = ReturnType<typeof createWsClient>
