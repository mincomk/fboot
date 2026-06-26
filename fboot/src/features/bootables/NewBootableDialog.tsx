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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useAppDispatch } from '@/store/hooks'
import { createBootable } from '@/store/slices/bootables'
import type { BootableKind } from '@/api'

export function NewBootableDialog() {
  const dispatch = useAppDispatch()
  const [open, setOpen] = useState(false)
  const [kind, setKind] = useState<BootableKind>('pxe')
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [cmdline, setCmdline] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [saving, setSaving] = useState(false)

  const submit = async () => {
    setError(null)
    setSaving(true)
    try {
      await dispatch(
        createBootable({
          kind,
          name: name.trim(),
          description: description.trim() || null,
          cmdline: kind === 'linux' ? cmdline.trim() || null : null,
        }),
      ).unwrap()
      setName('')
      setDescription('')
      setCmdline('')
      setKind('pxe')
      setOpen(false)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create bootable')
    } finally {
      setSaving(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus /> New bootable
        </Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New bootable</DialogTitle>
          <DialogDescription>
            Create a bootable, then upload its image files below.
          </DialogDescription>
        </DialogHeader>
        <form
          id="new-bootable-form"
          className="flex flex-col gap-4"
          onSubmit={(e) => {
            e.preventDefault()
            if (name.trim() && !saving) submit()
          }}
        >
          <div className="flex flex-col gap-1.5">
            <Label>Kind</Label>
            <Select value={kind} onValueChange={(v) => setKind(v as BootableKind)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="pxe">PXE (single boot image)</SelectItem>
                <SelectItem value="linux">Linux (kernel + initrd)</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="bname">Name</Label>
            <Input
              id="bname"
              placeholder="ipxe.efi / ubuntu-24.04"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <Label htmlFor="bdesc">Description (optional)</Label>
            <Input id="bdesc" value={description} onChange={(e) => setDescription(e.target.value)} />
          </div>
          {kind === 'linux' && (
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="bcmdline">Kernel param (optional)</Label>
              <Input
                id="bcmdline"
                placeholder="console=tty0 quiet"
                value={cmdline}
                onChange={(e) => setCmdline(e.target.value)}
              />
            </div>
          )}
          {error && <p className="text-sm text-destructive">{error}</p>}
        </form>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => setOpen(false)}>
            Cancel
          </Button>
          <Button type="submit" form="new-bootable-form" disabled={!name.trim() || saving}>
            {saving ? 'Creating…' : 'Create'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
