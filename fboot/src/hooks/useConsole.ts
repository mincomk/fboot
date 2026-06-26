import { useEffect, useRef, useState } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { ws } from '@/api'
import type { ConsoleConnection } from '@/api'

export type ConsoleStatus = 'connecting' | 'open' | 'closed'

export function useConsole(serverId: string | undefined) {
  const containerRef = useRef<HTMLDivElement | null>(null)
  const [status, setStatus] = useState<ConsoleStatus>('connecting')

  useEffect(() => {
    if (!serverId || !containerRef.current) return

    const term = new Terminal({
      convertEol: true,
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
      fontSize: 13,
      cursorBlink: true,
      theme: { background: '#0b0b0f' },
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(containerRef.current)

    const refit = () => {
      try {
        fit.fit()
      } catch {
        /* ignore */
      }
    }

    const raf = requestAnimationFrame(refit)
    const observer = new ResizeObserver(refit)
    observer.observe(containerRef.current)

    let connection: ConsoleConnection | null = null
    setStatus('connecting')
    connection = ws.connectConsole(serverId, {
      onData: (data) => {
        term.write(data)
        term.scrollToBottom()
      },
      onOpen: () => setStatus('open'),
      onClose: () => setStatus('closed'),
    })

    const disposable = term.onData((data) => connection?.send(data))

    return () => {
      cancelAnimationFrame(raf)
      observer.disconnect()
      disposable.dispose()
      connection?.close()
      term.dispose()
    }
  }, [serverId])

  return { containerRef, status }
}
