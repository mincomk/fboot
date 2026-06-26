import { useEffect, useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { fetchIpmi, saveIpmi } from '@/store/slices/servers'
import type { ServerView as View } from '@/hooks/useServers'

export function IpmiSection({ view }: { view: View }) {
  const dispatch = useAppDispatch()
  const id = view.server.id
  const creds = useAppSelector((s) => s.servers.ipmi[id])

  const [host, setHost] = useState('')
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [cipher, setCipher] = useState('')
  const [saved, setSaved] = useState(false)

  useEffect(() => {
    dispatch(fetchIpmi(id))
  }, [dispatch, id])

  useEffect(() => {
    if (!creds) return
    setHost(creds.host ?? '')
    setUsername(creds.username ?? '')
    setPassword(creds.password ?? '')
    setCipher(creds.cipher != null ? String(creds.cipher) : '')
  }, [creds])

  const save = async () => {
    setSaved(false)
    await dispatch(
      saveIpmi({
        id,
        creds: {
          host: host.trim() || null,
          username: username.trim() || null,
          password: password || null,
          cipher: cipher.trim() ? Number(cipher) : null,
        },
      }),
    ).unwrap()
    setSaved(true)
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>IPMI Credentials</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <p className="text-sm text-muted-foreground">
          Per-server overrides. Leave a field blank to use the server defaults
          (admin / admin, cipher 3). Host falls back to the discovered IP when blank.
        </p>
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault()
            save()
          }}
        >
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ipmi-host">BMC host / IP</Label>
            <Input id="ipmi-host" value={host} onChange={(e) => setHost(e.target.value)} />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ipmi-cipher">Cipher suite</Label>
            <Input
              id="ipmi-cipher"
              type="number"
              placeholder="3"
              value={cipher}
              onChange={(e) => setCipher(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ipmi-user">Username</Label>
            <Input
              id="ipmi-user"
              placeholder="admin"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="ipmi-pass">Password</Label>
            <Input
              id="ipmi-pass"
              type="password"
              placeholder="admin"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          </div>
        </div>
        <div className="flex items-center gap-3">
          <Button type="submit">Save credentials</Button>
          {saved && <span className="text-sm text-muted-foreground">Saved.</span>}
        </div>
        </form>
      </CardContent>
    </Card>
  )
}
