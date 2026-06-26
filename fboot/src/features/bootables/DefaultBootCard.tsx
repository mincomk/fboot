import { useEffect } from 'react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { fetchBootDefaults, updateBootDefaults } from '@/store/slices/bootables'
import type { BootableKind } from '@/api'

const NONE = '__none__'

export function DefaultBootCard() {
  const dispatch = useAppDispatch()
  const bootables = useAppSelector((s) => s.bootables.items)
  const defaults = useAppSelector((s) => s.bootables.defaults)

  useEffect(() => {
    dispatch(fetchBootDefaults())
  }, [dispatch])

  const pxeBootables = bootables.filter((b) => b.kind === 'pxe')
  const linuxBootables = bootables.filter((b) => b.kind === 'linux')

  const onChange = (kind: BootableKind, value: string) => {
    const id = value === NONE ? null : value
    dispatch(
      updateBootDefaults(kind === 'pxe' ? { pxe_bootable_id: id } : { linux_bootable_id: id }),
    )
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Default boot</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <p className="text-sm text-muted-foreground">
          Served to PXE clients whose MAC is not registered as a server. Registered servers
          always use their own boot configuration.
        </p>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="flex flex-col gap-2">
            <Label>Default PXE bootable</Label>
            <Select
              value={defaults.pxe_bootable_id ?? NONE}
              onValueChange={(value) => onChange('pxe', value)}
            >
              <SelectTrigger>
                <SelectValue placeholder="None" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NONE}>None</SelectItem>
                {pxeBootables.map((b) => (
                  <SelectItem key={b.id} value={b.id}>
                    {b.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-2">
            <Label>Default Linux bootable</Label>
            <Select
              value={defaults.linux_bootable_id ?? NONE}
              onValueChange={(value) => onChange('linux', value)}
            >
              <SelectTrigger>
                <SelectValue placeholder="None" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NONE}>None</SelectItem>
                {linuxBootables.map((b) => (
                  <SelectItem key={b.id} value={b.id}>
                    {b.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
