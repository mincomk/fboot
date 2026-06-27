import { useEffect, useRef, useState } from 'react'
import { Play, Square } from 'lucide-react'
import { ws } from '@/api'
import type { EventConnection, ScanOptions } from '@/api'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Progress } from '@/components/ui/progress'
import { Badge } from '@/components/ui/badge'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { scanDone, scanProgress, scanResult, scanStarted } from '@/store/slices/scan'

export function ScanPanel() {
  const dispatch = useAppDispatch()
  const { running, results, scanned, total } = useAppSelector((s) => s.scan)
  const connectionRef = useRef<EventConnection | null>(null)

  const [cidr, setCidr] = useState('')
  const [ipmi, setIpmi] = useState(true)
  const [ssh, setSsh] = useState(false)
  const [port, setPort] = useState('')

  useEffect(() => () => connectionRef.current?.close(), [])

  const start = () => {
    const ports = port
      .split(',')
      .map((p) => Number(p.trim()))
      .filter((p) => Number.isFinite(p) && p > 0)
    const options: ScanOptions = {
      cidr: cidr || undefined,
      probe_ipmi: ipmi,
      probe_ssh: ssh,
      ports: ports.length ? ports : undefined,
    }
    dispatch(scanStarted())
    connectionRef.current = ws.connectScan(
      options,
      (event) => {
        if (event.type === 'result') dispatch(scanResult(event))
        else if (event.type === 'progress')
          dispatch(scanProgress({ scanned: event.scanned, total: event.total }))
        else if (event.type === 'done') dispatch(scanDone())
      },
      () => dispatch(scanDone()),
    )
  }

  const stop = () => {
    connectionRef.current?.close()
    connectionRef.current = null
    dispatch(scanDone())
  }

  const pct = total > 0 ? (scanned / total) * 100 : running ? 8 : 0

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader>
          <CardTitle>Network Scan</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-4">
          <form
            className="flex flex-col gap-4"
            onSubmit={(e) => {
              e.preventDefault()
              if (!running && cidr.trim()) start()
            }}
          >
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="cidr">CIDR range</Label>
              <Input
                id="cidr"
                placeholder="192.168.1.0/24"
                value={cidr}
                onChange={(e) => setCidr(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="port">Custom ports</Label>
              <Input
                id="port"
                placeholder="e.g. 22,623"
                value={port}
                onChange={(e) => setPort(e.target.value)}
              />
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-6">
            <div className="flex items-center gap-2">
              <Switch id="ipmi" checked={ipmi} onCheckedChange={setIpmi} />
              <Label htmlFor="ipmi">IPMI</Label>
            </div>
            <div className="flex items-center gap-2">
              <Switch id="ssh" checked={ssh} onCheckedChange={setSsh} />
              <Label htmlFor="ssh">SSH</Label>
            </div>
            <div className="ml-auto">
              {running ? (
                <Button type="button" variant="destructive" onClick={stop}>
                  <Square /> Stop
                </Button>
              ) : (
                <Button type="submit" disabled={!cidr.trim()}>
                  <Play /> Start scan
                </Button>
              )}
            </div>
          </div>
          </form>
          {(running || total > 0) && (
            <div className="flex flex-col gap-1.5">
              <Progress value={pct} />
              <p className="text-xs text-muted-foreground">
                {total > 0 ? `${scanned} / ${total} scanned` : 'Scanning…'}
              </p>
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row items-center justify-between">
          <CardTitle>Results</CardTitle>
          <Badge variant="secondary">{results.length}</Badge>
        </CardHeader>
        <CardContent>
          {results.length === 0 ? (
            <p className="text-sm text-muted-foreground">No hosts discovered yet.</p>
          ) : (
            <div className="overflow-x-auto rounded-md border">
              <table className="w-full text-left text-sm">
                <thead className="bg-muted/50 text-xs uppercase text-muted-foreground">
                  <tr>
                    <th className="px-3 py-2 font-medium">IP</th>
                    <th className="px-3 py-2 font-medium">MAC</th>
                    <th className="px-3 py-2 font-medium">Hostname</th>
                    <th className="px-3 py-2 font-medium">Vendor</th>
                    <th className="px-3 py-2 font-medium">Services</th>
                    <th className="px-3 py-2 font-medium">Open ports</th>
                  </tr>
                </thead>
                <tbody>
                  {results.map((r) => (
                    <tr key={r.ip} className="border-b last:border-0">
                      <td className="px-3 py-2 font-mono">{r.ip}</td>
                      <td className="px-3 py-2 font-mono text-muted-foreground">{r.mac ?? '—'}</td>
                      <td className="px-3 py-2">{r.hostname ?? '—'}</td>
                      <td className="px-3 py-2 text-muted-foreground">{r.vendor ?? '—'}</td>
                      <td className="px-3 py-2">
                        <div className="flex gap-1">
                          {r.ipmi && <Badge variant="success">IPMI</Badge>}
                          {r.ssh && <Badge variant="secondary">SSH</Badge>}
                          {!r.ipmi && !r.ssh && <span className="text-muted-foreground">—</span>}
                        </div>
                      </td>
                      <td className="px-3 py-2 font-mono text-muted-foreground">
                        {r.open_ports.length ? r.open_ports.join(', ') : '—'}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
