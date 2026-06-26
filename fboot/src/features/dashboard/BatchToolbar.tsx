import { HardDrive, Network, Power, PowerOff, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { powerAction, updateBootConfig } from '@/store/slices/servers'
import { clearSelection } from '@/store/slices/ui'

export function BatchToolbar() {
  const dispatch = useAppDispatch()
  const selected = useAppSelector((s) => s.ui.selectedServerIds)
  const bootables = useAppSelector((s) => s.bootables.items)

  if (selected.length === 0) return null

  const pxeBootables = bootables.filter((b) => b.kind === 'pxe')
  const linuxBootables = bootables.filter((b) => b.kind === 'linux')
  const each = (fn: (id: string) => void) => selected.forEach(fn)

  return (
    <div className="sticky bottom-4 z-10 mx-auto flex w-fit max-w-full flex-wrap items-center gap-3 rounded-xl border bg-card/95 px-4 py-2.5 shadow-lg backdrop-blur">
      <span className="text-sm font-medium">{selected.length} selected</span>

      <Button
        size="sm"
        variant="success"
        onClick={() => each((id) => dispatch(powerAction({ id, action: 'on' })))}
      >
        <Power /> On
      </Button>
      <Button
        size="sm"
        variant="destructive"
        onClick={() => each((id) => dispatch(powerAction({ id, action: 'off' })))}
      >
        <PowerOff /> Off
      </Button>

      <Button
        size="sm"
        className="bg-warning text-warning-foreground hover:bg-warning/90"
        onClick={() =>
          each((id) => dispatch(updateBootConfig({ id, patch: { boot_pxe: true } })))
        }
      >
        <Network /> Boot PXE
      </Button>
      <Button
        size="sm"
        variant="secondary"
        onClick={() =>
          each((id) => dispatch(updateBootConfig({ id, patch: { boot_pxe: false } })))
        }
      >
        <HardDrive /> Boot Local
      </Button>

      <Select
        onValueChange={(bootableId) =>
          each((id) => dispatch(updateBootConfig({ id, patch: { pxe_bootable_id: bootableId } })))
        }
      >
        <SelectTrigger className="h-8 w-44">
          <SelectValue placeholder="Set PXE bootable" />
        </SelectTrigger>
        <SelectContent>
          {pxeBootables.map((b) => (
            <SelectItem key={b.id} value={b.id}>
              {b.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Select
        onValueChange={(bootableId) =>
          each((id) => dispatch(updateBootConfig({ id, patch: { linux_bootable_id: bootableId } })))
        }
      >
        <SelectTrigger className="h-8 w-44">
          <SelectValue placeholder="Set Linux bootable" />
        </SelectTrigger>
        <SelectContent>
          {linuxBootables.map((b) => (
            <SelectItem key={b.id} value={b.id}>
              {b.name}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      <Button size="icon" variant="ghost" className="size-8" onClick={() => dispatch(clearSelection())}>
        <X />
      </Button>
    </div>
  )
}
