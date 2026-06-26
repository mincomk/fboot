import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'

export interface BootModeToggleProps {
  bootPxe: boolean
  onChange: (pxe: boolean) => void
}

export function BootModeToggle({ bootPxe, onChange }: BootModeToggleProps) {
  const click = (pxe: boolean) => (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()
    if (pxe !== bootPxe) onChange(pxe)
  }

  return (
    <div className="inline-flex items-center rounded-md border p-0.5">
      <Button
        size="sm"
        variant="ghost"
        className={cn('h-6 px-2 text-xs', bootPxe && 'bg-warning text-warning-foreground hover:bg-warning/90')}
        onClick={click(true)}
      >
        PXE
      </Button>
      <Button
        size="sm"
        variant="ghost"
        className={cn('h-6 px-2 text-xs', !bootPxe && 'bg-secondary text-secondary-foreground')}
        onClick={click(false)}
      >
        Local
      </Button>
    </div>
  )
}
