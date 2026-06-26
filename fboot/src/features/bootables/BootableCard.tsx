import { useEffect, useState } from 'react'
import { Trash2, Upload } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { useAppDispatch } from '@/store/hooks'
import { deleteBootable, updateBootable, uploadBootableFile } from '@/store/slices/bootables'
import { formatBytes, shortId } from '@/lib/format'
import type { Bootable, BootableRole } from '@/api'

const ROLES: Record<Bootable['kind'], BootableRole[]> = {
  pxe: ['image'],
  linux: ['kernel', 'initrd'],
}

function FileRow({ bootable, role }: { bootable: Bootable; role: BootableRole }) {
  const dispatch = useAppDispatch()
  const [busy, setBusy] = useState(false)
  const existing = bootable.files.find((f) => f.role === role)

  const onPick = async (file: File | undefined) => {
    if (!file) return
    setBusy(true)
    try {
      await dispatch(uploadBootableFile({ id: bootable.id, role, file })).unwrap()
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="flex items-center justify-between gap-3 rounded-md border px-3 py-2">
      <div className="min-w-0">
        <p className="text-sm font-medium capitalize">{role}</p>
        <p className="truncate text-xs text-muted-foreground">
          {existing
            ? existing.source === 'url'
              ? existing.url
              : `${shortId(existing.key ?? '')} · ${formatBytes(existing.size)}`
            : 'not uploaded'}
        </p>
      </div>
      <Button asChild size="sm" variant="outline" disabled={busy}>
        <label className="cursor-pointer">
          <Upload /> {busy ? 'Uploading…' : existing ? 'Replace' : 'Upload'}
          <input
            type="file"
            className="hidden"
            onChange={(e) => onPick(e.target.files?.[0])}
          />
        </label>
      </Button>
    </div>
  )
}

function KernelParamRow({ bootable }: { bootable: Bootable }) {
  const dispatch = useAppDispatch()
  const [cmdline, setCmdline] = useState(bootable.cmdline ?? '')
  useEffect(() => {
    setCmdline(bootable.cmdline ?? '')
  }, [bootable.cmdline])

  return (
    <form
      className="flex flex-col gap-1.5"
      onSubmit={(e) => {
        e.preventDefault()
        dispatch(updateBootable({ id: bootable.id, patch: { cmdline: cmdline || null } }))
      }}
    >
      <Label htmlFor={`cmdline-${bootable.id}`}>Kernel param</Label>
      <div className="flex gap-2">
        <Input
          id={`cmdline-${bootable.id}`}
          placeholder="console=tty0 quiet"
          value={cmdline}
          onChange={(e) => setCmdline(e.target.value)}
        />
        <Button type="submit" variant="outline" disabled={cmdline === (bootable.cmdline ?? '')}>
          Save
        </Button>
      </div>
    </form>
  )
}

export function BootableCard({ bootable }: { bootable: Bootable }) {
  const dispatch = useAppDispatch()

  return (
    <Card>
      <CardHeader className="flex flex-row items-start justify-between gap-2">
        <div className="min-w-0">
          <CardTitle className="flex items-center gap-2">
            <span className="truncate">{bootable.name}</span>
            <Badge variant={bootable.kind === 'pxe' ? 'default' : 'secondary'}>
              {bootable.kind.toUpperCase()}
            </Badge>
          </CardTitle>
          {bootable.description && (
            <p className="mt-1 text-sm text-muted-foreground">{bootable.description}</p>
          )}
        </div>
        <Button
          size="icon"
          variant="ghost"
          className="text-destructive"
          onClick={() => dispatch(deleteBootable(bootable.id))}
        >
          <Trash2 />
        </Button>
      </CardHeader>
      <CardContent className="flex flex-col gap-2">
        {ROLES[bootable.kind].map((role) => (
          <FileRow key={role} bootable={bootable} role={role} />
        ))}
        {bootable.kind === 'linux' && <KernelParamRow bootable={bootable} />}
      </CardContent>
    </Card>
  )
}
