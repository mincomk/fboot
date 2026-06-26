import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { Textarea } from '@/components/ui/textarea'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { fetchBootConfig, setBootDev, updateBootConfig } from '@/store/slices/servers'
import { api } from '@/api'
import type { BootDev } from '@/api'
import type { ServerView as View } from '@/hooks/useServers'

const BOOT_DEVS: BootDev[] = ['none', 'pxe', 'disk', 'cdrom', 'bios']

export function BootManagementSection({ view }: { view: View }) {
  const dispatch = useAppDispatch()
  const bootables = useAppSelector((s) => s.bootables.items)
  const id = view.server.id
  const config = view.bootConfig

  useEffect(() => {
    if (!config) dispatch(fetchBootConfig(id))
  }, [dispatch, id, config])

  const pxeBootables = bootables.filter((b) => b.kind === 'pxe')
  const linuxBootables = bootables.filter((b) => b.kind === 'linux')

  const [cmdlineOverride, setCmdlineOverride] = useState(config?.cmdline_override ?? '')
  useEffect(() => {
    setCmdlineOverride(config?.cmdline_override ?? '')
  }, [config?.cmdline_override])

  const [cmdlineAppend, setCmdlineAppend] = useState(config?.cmdline_append ?? '')
  useEffect(() => {
    setCmdlineAppend(config?.cmdline_append ?? '')
  }, [config?.cmdline_append])

  const [script, setScript] = useState<string | null>(null)
  const [scriptLoading, setScriptLoading] = useState(false)
  const loadScript = async () => {
    setScriptLoading(true)
    try {
      const res = await api.boot.ipxe(id)
      setScript(res.script)
    } finally {
      setScriptLoading(false)
    }
  }

  const [ipxeScript, setIpxeScript] = useState(config?.ipxe_script ?? '')
  useEffect(() => {
    setIpxeScript(config?.ipxe_script ?? '')
  }, [config?.ipxe_script])
  const overridden = (config?.ipxe_script ?? '') !== ''
  const saveOverride = () =>
    dispatch(updateBootConfig({ id, patch: { ipxe_script: ipxeScript || null } }))
  const clearOverride = () => {
    setIpxeScript('')
    dispatch(updateBootConfig({ id, patch: { ipxe_script: null } }))
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Boot Management</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-6">
        <div className="flex items-center justify-between">
          <div>
            <Label>Boot PXE</Label>
            <p className="text-xs text-muted-foreground">
              Serve the PXE bootable to this server on next boot.
            </p>
          </div>
          <Switch
            checked={config?.boot_pxe ?? false}
            onCheckedChange={(checked) =>
              dispatch(updateBootConfig({ id, patch: { boot_pxe: checked } }))
            }
          />
        </div>

        <div className="flex flex-col gap-2">
          <Label>Boot device</Label>
          <Select onValueChange={(dev) => dispatch(setBootDev({ id, dev: dev as BootDev }))}>
            <SelectTrigger>
              <SelectValue placeholder="Set one-time bootdev" />
            </SelectTrigger>
            <SelectContent>
              {BOOT_DEVS.map((dev) => (
                <SelectItem key={dev} value={dev}>
                  {dev.toUpperCase()}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="flex flex-col gap-2">
          <Label>PXE bootable</Label>
          <Select
            value={config?.pxe_bootable_id ?? undefined}
            onValueChange={(value) =>
              dispatch(updateBootConfig({ id, patch: { pxe_bootable_id: value } }))
            }
          >
            <SelectTrigger>
              <SelectValue placeholder="Select PXE bootable" />
            </SelectTrigger>
            <SelectContent>
              {pxeBootables.map((b) => (
                <SelectItem key={b.id} value={b.id}>
                  {b.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="flex flex-col gap-2">
          <Label>Linux bootable</Label>
          <Select
            value={config?.linux_bootable_id ?? undefined}
            onValueChange={(value) =>
              dispatch(updateBootConfig({ id, patch: { linux_bootable_id: value } }))
            }
          >
            <SelectTrigger>
              <SelectValue placeholder="Select Linux bootable" />
            </SelectTrigger>
            <SelectContent>
              {linuxBootables.map((b) => (
                <SelectItem key={b.id} value={b.id}>
                  {b.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="flex flex-col gap-2">
          <Label htmlFor="cmdline-override">Kernel cmdline override</Label>
          <p className="text-xs text-muted-foreground">
            Replaces the Linux bootable's kernel param for this server. Leave empty to use the
            bootable's param.
          </p>
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault()
              dispatch(updateBootConfig({ id, patch: { cmdline_override: cmdlineOverride || null } }))
            }}
          >
            <Input
              id="cmdline-override"
              placeholder="console=ttyS0 ip=dhcp"
              value={cmdlineOverride}
              onChange={(e) => setCmdlineOverride(e.target.value)}
            />
            <Button type="submit" variant="outline">
              Save
            </Button>
          </form>
        </div>

        <div className="flex flex-col gap-2">
          <Label htmlFor="cmdline-append">Kernel cmdline append</Label>
          <p className="text-xs text-muted-foreground">
            Added on top of the effective kernel param (after the override or the bootable's param).
          </p>
          <form
            className="flex gap-2"
            onSubmit={(e) => {
              e.preventDefault()
              dispatch(updateBootConfig({ id, patch: { cmdline_append: cmdlineAppend || null } }))
            }}
          >
            <Input
              id="cmdline-append"
              placeholder="ip=dhcp"
              value={cmdlineAppend}
              onChange={(e) => setCmdlineAppend(e.target.value)}
            />
            <Button type="submit" variant="outline">
              Save
            </Button>
          </form>
        </div>

        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <Label htmlFor="ipxe-override">iPXE script override</Label>
            <span className="text-xs text-muted-foreground">
              {overridden ? 'Custom script served' : 'Using generated script'}
            </span>
          </div>
          <p className="text-xs text-muted-foreground">
            Served verbatim at <code>/boot/{'{mac}'}.ipxe</code> instead of the generated script.
            Leave empty to use the generated one.
          </p>
          <Textarea
            id="ipxe-override"
            className="font-mono text-xs"
            placeholder={'#!ipxe\nkernel ... initrd=initrd.img\ninitrd ...\nboot'}
            value={ipxeScript}
            onChange={(e) => setIpxeScript(e.target.value)}
          />
          <div className="flex gap-2">
            <Button
              variant="outline"
              onClick={saveOverride}
              disabled={ipxeScript === (config?.ipxe_script ?? '')}
            >
              Save override
            </Button>
            <Button variant="ghost" onClick={clearOverride} disabled={!overridden}>
              Reset to generated
            </Button>
          </div>
        </div>

        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between">
            <Label>Generated iPXE script</Label>
            <div className="flex gap-2">
              {script != null && (
                <Button variant="ghost" size="sm" onClick={() => setIpxeScript(script)}>
                  Copy to override
                </Button>
              )}
              <Button variant="outline" size="sm" disabled={scriptLoading} onClick={loadScript}>
                {scriptLoading ? 'Loading…' : script ? 'Refresh' : 'Show'}
              </Button>
            </div>
          </div>
          {script != null && (
            <pre className="overflow-x-auto rounded-md border bg-muted/40 p-3 font-mono text-xs">
              {script}
            </pre>
          )}
        </div>
      </CardContent>
    </Card>
  )
}
