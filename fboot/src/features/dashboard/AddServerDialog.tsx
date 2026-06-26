import { useState } from 'react'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useAppDispatch } from '@/store/hooks'
import { createServer } from '@/store/slices/servers'

export function AddServerDialog() {
  const dispatch = useAppDispatch()
  const [open, setOpen] = useState(false)
  const [mac, setMac] = useState('')
  const [ipmiMac, setIpmiMac] = useState('')
  const [name, setName] = useState('')
  const [hostname, setHostname] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  const valid = ipmiMac.trim().length > 0 && name.trim().length > 0

  const submit = async () => {
    setError(null)
    setSaving(true)
    try {
      await dispatch(
        createServer({
          primary_mac: mac.trim() || null,
          ipmi_mac: ipmiMac.trim(),
          friendly_name: name.trim(),
          hostname: hostname.trim() || null,
        }),
      ).unwrap()
      setMac('')
      setIpmiMac('')
      setName('')
      setHostname('')
      setOpen(false)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create server')
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus /> Add server
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add server</DialogTitle>
          <DialogDescription>Register a server by its MAC address.</DialogDescription>
        </DialogHeader>
        <form
          id="add-server-form"
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault()
            if (valid && !saving) submit()
          }}
        >
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ipmi-mac">IPMI MAC address</Label>
            <Input
              id="ipmi-mac"
              placeholder="aa:bb:cc:dd:ee:ff"
              value={ipmiMac}
              onChange={(e) => setIpmiMac(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="mac">Primary MAC address (optional)</Label>
            <Input
              id="mac"
              placeholder="aa:bb:cc:dd:ee:ff"
              value={mac}
              onChange={(e) => setMac(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="name">Friendly name</Label>
            <Input
              id="name"
              placeholder="node-01"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="hostname">Hostname (optional)</Label>
            <Input
              id="hostname"
              placeholder="node-01.lan"
              value={hostname}
              onChange={(e) => setHostname(e.target.value)}
            />
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
        </form>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button type="submit" form="add-server-form" disabled={!valid || saving}>
            {saving ? 'Adding…' : 'Add server'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
