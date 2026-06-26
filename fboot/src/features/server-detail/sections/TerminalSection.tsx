import { useEffect } from 'react'
import { Square, Users } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { api } from '@/api'
import { useConsole } from '@/hooks/useConsole'
import type { ServerView } from '@/hooks/useServers'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { consoleStatusChanged } from '@/store/slices/console'

const STATUS_VARIANT = {
  connecting: 'warning',
  open: 'success',
  closed: 'muted',
} as const

export function TerminalSection({ view }: { view: ServerView }) {
  const id = view.server.id
  const { containerRef, status } = useConsole(id)
  const dispatch = useAppDispatch()
  const session = useAppSelector((s) => s.console.byServer[id]) ?? null

  useEffect(() => {
    api.console
      .status(id)
      .then((next) => dispatch(consoleStatusChanged({ server_id: id, status: next })))
      .catch(() => {})
  }, [id, dispatch])

  const kill = async () => {
    const next = await api.console.kill(id)
    dispatch(consoleStatusChanged({ server_id: id, status: next }))
  }

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between">
        <div className="flex items-center gap-2">
          <CardTitle>Serial Console</CardTitle>
          {session?.running && (
            <Badge variant="muted" className="gap-1">
              <Users className="size-3" /> {session.clients}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Badge variant={STATUS_VARIANT[status]}>{status}</Badge>
          {session?.running && (
            <Button size="sm" variant="outline" onClick={kill}>
              <Square /> Kill session
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        <p className="text-xs text-muted-foreground">
          This console runs in the background on the server and is shared by all viewers; it
          keeps running after you leave. Reopen to reattach, or kill it to end the session.
        </p>
        <div className="h-[420px] w-full overflow-hidden rounded-md border bg-[#0b0b0f] p-2">
          <div ref={containerRef} className="h-full w-full" />
        </div>
      </CardContent>
    </Card>
  )
}
