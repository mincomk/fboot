import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { Pencil, Plus, Trash2, X } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import { MetaTable } from '@/components/shared/MetaTable'
import { StatusBadge } from '@/components/shared/StatusBadge'
import { formatMac, formatRelative } from '@/lib/format'
import { useAppDispatch } from '@/store/hooks'
import {
  deleteServerMeta,
  removeServer,
  setServerMeta,
  updateServer,
} from '@/store/slices/servers'
import type { ServerView } from '@/hooks/useServers'

export function InfoSection({ view }: { view: ServerView }) {
  const { server, status, stats } = view
  const dispatch = useAppDispatch()
  const navigate = useNavigate()
  const power = stats?.power_status ?? 'unknown'
  const metaEntries = Object.entries(server.metadata ?? {})

  const [editing, setEditing] = useState(false)
  const [name, setName] = useState(server.friendly_name)
  const [hostname, setHostname] = useState(server.hostname ?? '')
  const [primaryMac, setPrimaryMac] = useState(server.primary_mac ?? '')
  const [ipmiMac, setIpmiMac] = useState(server.ipmi_mac)
  const [metaKey, setMetaKey] = useState('')
  const [metaValue, setMetaValue] = useState('')

  const saveInfo = async () => {
    await dispatch(
      updateServer({
        id: server.id,
        patch: {
          friendly_name: name.trim(),
          hostname: hostname.trim() || null,
          primary_mac: primaryMac.trim() || null,
          ipmi_mac: ipmiMac.trim(),
        },
      }),
    ).unwrap()
    setEditing(false)
  }

  const addMeta = async () => {
    if (!metaKey.trim()) return
    await dispatch(setServerMeta({ id: server.id, key: metaKey.trim(), value: metaValue })).unwrap()
    setMetaKey('')
    setMetaValue('')
  }

  const onDelete = async () => {
    await dispatch(removeServer(server.id)).unwrap()
    navigate('/')
  }

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>Server Info</CardTitle>
          {!editing && (
            <Button size="sm" variant="outline" onClick={() => setEditing(true)}>
              <Pencil /> Edit
            </Button>
          )}
        </CardHeader>
        <CardContent>
          {editing ? (
            <form
              className="flex flex-col gap-4"
              onSubmit={(e) => {
                e.preventDefault()
                if (name.trim() && ipmiMac.trim()) saveInfo()
              }}
            >
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="fname">Friendly name</Label>
                <Input id="fname" value={name} onChange={(e) => setName(e.target.value)} />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="fhost">Hostname</Label>
                <Input id="fhost" value={hostname} onChange={(e) => setHostname(e.target.value)} />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="fipmimac">IPMI MAC</Label>
                <Input
                  id="fipmimac"
                  placeholder="aa:bb:cc:dd:ee:ff"
                  value={ipmiMac}
                  onChange={(e) => setIpmiMac(e.target.value)}
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="fpmac">Primary MAC (optional)</Label>
                <Input
                  id="fpmac"
                  placeholder="aa:bb:cc:dd:ee:ff"
                  value={primaryMac}
                  onChange={(e) => setPrimaryMac(e.target.value)}
                />
              </div>
              <div className="flex gap-2">
                <Button type="submit" disabled={!name.trim() || !ipmiMac.trim()}>
                  Save
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => {
                    setName(server.friendly_name)
                    setHostname(server.hostname ?? '')
                    setPrimaryMac(server.primary_mac ?? '')
                    setIpmiMac(server.ipmi_mac)
                    setEditing(false)
                  }}
                >
                  Cancel
                </Button>
              </div>
            </form>
          ) : (
            <MetaTable
              rows={[
                { label: 'Friendly name', value: server.friendly_name },
                { label: 'Hostname', value: server.hostname ?? '—' },
                {
                  label: 'Primary MAC',
                  value: <span className="font-mono">{formatMac(server.primary_mac)}</span>,
                },
                { label: 'Primary IP', value: status?.ip ?? '—' },
                {
                  label: 'IPMI MAC',
                  value: <span className="font-mono">{formatMac(server.ipmi_mac)}</span>,
                },
                { label: 'IPMI IP', value: status?.ipmi_ip ?? '—' },
                { label: 'Power', value: <StatusBadge status={power} /> },
                { label: 'IPMI', value: status?.ipmi_reachable ? 'Reachable' : 'Unreachable' },
                { label: 'Online', value: status?.online ? 'Yes' : 'No' },
                { label: 'Created', value: formatRelative(server.created_at) },
                { label: 'Updated', value: formatRelative(server.updated_at) },
              ]}
            />
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Metadata</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          {metaEntries.length === 0 ? (
            <p className="text-sm text-muted-foreground">No metadata.</p>
          ) : (
            <div className="flex flex-col divide-y">
              {metaEntries.map(([key, value]) => (
                <div key={key} className="flex items-center justify-between gap-3 py-2">
                  <div className="min-w-0">
                    <p className="text-sm font-medium">{key}</p>
                    <p className="truncate text-sm text-muted-foreground">{value}</p>
                  </div>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="size-8 text-muted-foreground"
                    onClick={() => dispatch(deleteServerMeta({ id: server.id, key }))}
                  >
                    <X />
                  </Button>
                </div>
              ))}
            </div>
          )}
          <form
            className="flex items-end gap-2"
            onSubmit={(e) => {
              e.preventDefault()
              addMeta()
            }}
          >
            <div className="flex flex-1 flex-col gap-1.5">
              <Label htmlFor="mkey">Key</Label>
              <Input id="mkey" value={metaKey} onChange={(e) => setMetaKey(e.target.value)} />
            </div>
            <div className="flex flex-1 flex-col gap-1.5">
              <Label htmlFor="mval">Value</Label>
              <Input id="mval" value={metaValue} onChange={(e) => setMetaValue(e.target.value)} />
            </div>
            <Button type="submit" variant="outline" disabled={!metaKey.trim()}>
              <Plus /> Add
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card className="border-destructive/40">
        <CardHeader>
          <CardTitle className="text-destructive">Danger zone</CardTitle>
        </CardHeader>
        <CardContent>
          <Dialog>
            <DialogTrigger asChild>
              <Button variant="destructive">
                <Trash2 /> Delete server
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Delete {server.friendly_name}?</DialogTitle>
                <DialogDescription>
                  This removes the server, its boot config, metadata and IPMI override. This
                  cannot be undone.
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <Button variant="destructive" onClick={onDelete}>
                  Delete
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        </CardContent>
      </Card>
    </div>
  )
}
