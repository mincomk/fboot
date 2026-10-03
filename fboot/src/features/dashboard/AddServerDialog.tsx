import { useState } from 'react'
import { Plus } from 'lucide-react'
import { toast } from 'sonner'
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
import { createServer, saveIpmi } from '@/store/slices/servers'

export function AddServerDialog() {
  const dispatch = useAppDispatch()
  const [open, setOpen] = useState(false)
  const [mac, setMac] = useState('')
  const [ipmiMac, setIpmiMac] = useState('')
  const [name, setName] = useState('')
  const [hostname, setHostname] = useState('')
  // Optional per-device BMC credentials, saved right after the server is created.
  const [ipmiHost, setIpmiHost] = useState('')
  const [ipmiUser, setIpmiUser] = useState('')
  const [ipmiPass, setIpmiPass] = useState('')
  const [ipmiCipher, setIpmiCipher] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  const valid = name.trim().length > 0

  const reset = () => {
    setMac('')
    setIpmiMac('')
    setName('')
    setHostname('')
    setIpmiHost('')
    setIpmiUser('')
    setIpmiPass('')
    setIpmiCipher('')
  }

  const submit = async () => {
    setError(null)
    setSaving(true)
    try {
      const server = await dispatch(
        createServer({
          primary_mac: mac.trim() || null,
          ipmi_mac: ipmiMac.trim() || null,
          friendly_name: name.trim(),
          hostname: hostname.trim() || null,
        }),
      ).unwrap()

      const wantsIpmi = Boolean(
        ipmiHost.trim() || ipmiUser.trim() || ipmiPass || ipmiCipher.trim(),
      )
      if (wantsIpmi) {
        try {
          await dispatch(
            saveIpmi({
              id: server.id,
              creds: {
                host: ipmiHost.trim() || null,
                username: ipmiUser.trim() || null,
                password: ipmiPass || null,
                cipher: ipmiCipher.trim() ? Number(ipmiCipher) : null,
              },
            }),
          ).unwrap()
        } catch (e) {
          // The server exists either way; the credentials can be fixed on its IPMI page.
          toast.warning('Server added, but its IPMI credentials were not saved', {
            description: e instanceof Error ? e.message : undefined,
          })
        }
      }

      reset()
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
            <Label htmlFor="ipmi-mac">IPMI MAC address (optional)</Label>
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

          <div className="flex flex-col gap-3 rounded-md border p-3">
            <p className="text-sm font-medium">BMC credentials (optional)</p>
            <p className="text-xs text-muted-foreground">
              Set these if this device&apos;s IPMI user or password differs from the server-wide
              defaults. Everything can be changed later on the server&apos;s IPMI page.
            </p>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="ipmi-host">IPMI host / IP</Label>
                <Input
                  id="ipmi-host"
                  placeholder="auto-discovered"
                  value={ipmiHost}
                  onChange={(e) => setIpmiHost(e.target.value)}
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="ipmi-cipher">Cipher suite</Label>
                <Input
                  id="ipmi-cipher"
                  type="number"
                  placeholder="default"
                  value={ipmiCipher}
                  onChange={(e) => setIpmiCipher(e.target.value)}
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="ipmi-user">IPMI username</Label>
                <Input
                  id="ipmi-user"
                  placeholder="default"
                  value={ipmiUser}
                  onChange={(e) => setIpmiUser(e.target.value)}
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="ipmi-pass">IPMI password</Label>
                <Input
                  id="ipmi-pass"
                  type="password"
                  placeholder="default"
                  value={ipmiPass}
                  onChange={(e) => setIpmiPass(e.target.value)}
                />
              </div>
            </div>
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
