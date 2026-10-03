import { useEffect, useState } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { toast } from 'sonner'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { clearIpmi, fetchIpmi, powerAction, saveIpmi } from '@/store/slices/servers'
import type { ServerView as View } from '@/hooks/useServers'

function message(e: unknown): string | undefined {
  return e instanceof Error ? e.message : typeof e === 'string' ? e : undefined
}

type LoadState = 'loading' | 'ready' | 'error'

export function IpmiSection({ view }: { view: View }) {
  const dispatch = useAppDispatch()
  const id = view.server.id
  const creds = useAppSelector((s) => s.servers.ipmi[id])
  const reachable = view.status?.ipmi_reachable ?? false

  const [host, setHost] = useState('')
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [cipher, setCipher] = useState('')
  const [busy, setBusy] = useState(false)
  // Saving is only allowed once the stored override has actually been read, so a
  // failed fetch can never be followed by a Save that blanks the credentials.
  const [load, setLoad] = useState<LoadState>('loading')
  const [attempt, setAttempt] = useState(0)
  // A probe that answers overrides a stale "not answering" status.
  const [probeOk, setProbeOk] = useState(false)

  useEffect(() => {
    let cancelled = false
    setLoad('loading')
    setProbeOk(false)
    dispatch(fetchIpmi(id))
      .unwrap()
      .then(() => {
        if (!cancelled) setLoad('ready')
      })
      .catch(() => {
        if (!cancelled) setLoad('error')
      })
    return () => {
      cancelled = true
    }
  }, [dispatch, id, attempt])

  useEffect(() => {
    if (!creds) return
    setHost(creds.host ?? '')
    setUsername(creds.username ?? '')
    setPassword(creds.password ?? '')
    setCipher(creds.cipher != null && creds.cipher !== 0 ? String(creds.cipher) : '')
  }, [creds])

  const editable = load === 'ready' && !busy

  const save = async () => {
    setBusy(true)
    try {
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
      toast.success('IPMI credentials saved')
    } catch (e) {
      toast.error('Could not save IPMI credentials', { description: message(e) })
    } finally {
      setBusy(false)
    }
  }

  const clear = async () => {
    setBusy(true)
    try {
      await dispatch(clearIpmi(id)).unwrap()
      setHost('')
      setUsername('')
      setPassword('')
      setCipher('')
      toast.success('Reverted to the server-wide IPMI defaults')
    } catch (e) {
      toast.error('Could not clear the IPMI override', { description: message(e) })
    } finally {
      setBusy(false)
    }
  }

  const test = async () => {
    setBusy(true)
    try {
      const res = await dispatch(powerAction({ id, action: 'status' })).unwrap()
      setProbeOk(true)
      toast.success(`IPMI answered — power is ${res.power}`)
    } catch (e) {
      toast.error('IPMI did not answer with these credentials', { description: message(e) })
    } finally {
      setBusy(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>IPMI Credentials</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <p className="text-sm text-muted-foreground">
          Per-device overrides, used for this server only. Leave a field blank (or clear it) to
          fall back to the server-wide defaults — the BMC host then also falls back to the address
          discovered for the IPMI MAC.
        </p>
        {load === 'error' && (
          <div className="flex flex-wrap items-center gap-3 rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm">
            <span>Could not read the stored IPMI credentials for this server.</span>
            <Button type="button" variant="outline" onClick={() => setAttempt((n) => n + 1)}>
              Retry
            </Button>
          </div>
        )}
        {load !== 'error' && !reachable && !probeOk && (
          <p className="rounded-md border border-amber-500/40 bg-amber-500/10 p-3 text-sm">
            IPMI is not answering for this server. Set its BMC host, username and password here,
            then press <span className="font-medium">Test connection</span>.
          </p>
        )}
        <form
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault()
            if (editable) save()
          }}
        >
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ipmi-host">BMC host / IP</Label>
              <Input
                id="ipmi-host"
                placeholder="auto-discovered"
                value={host}
                disabled={!editable}
                onChange={(e) => setHost(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ipmi-cipher">Cipher suite</Label>
              <Input
                id="ipmi-cipher"
                type="number"
                min={1}
                max={255}
                step={1}
                placeholder="default"
                value={cipher}
                disabled={!editable}
                onChange={(e) => setCipher(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ipmi-user">Username</Label>
              <Input
                id="ipmi-user"
                placeholder="default"
                value={username}
                disabled={!editable}
                onChange={(e) => setUsername(e.target.value)}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="ipmi-pass">Password</Label>
              <Input
                id="ipmi-pass"
                type="password"
                placeholder="default"
                value={password}
                disabled={!editable}
                onChange={(e) => setPassword(e.target.value)}
              />
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <Button type="submit" disabled={!editable}>
              Save credentials
            </Button>
            <Button type="button" variant="outline" disabled={!editable} onClick={test}>
              Test connection
            </Button>
            <Button type="button" variant="ghost" disabled={!editable} onClick={clear}>
              Clear override
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  )
}
