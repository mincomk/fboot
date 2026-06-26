import { Power, PowerOff, RotateCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { PowerAction, PowerStatus } from '@/api'

export interface PowerButtonProps {
  status: PowerStatus
  onAction: (action: PowerAction) => void
  pending?: PowerAction | null
  size?: 'sm' | 'default'
}

export function PowerButton({ status, onAction, pending, size = 'sm' }: PowerButtonProps) {
  const click = (action: PowerAction) => (e: React.MouseEvent) => {
    e.preventDefault()
    e.stopPropagation()
    onAction(action)
  }

  return (
    <div className="flex items-center gap-2">
      <Button
        size={size}
        variant="success"
        disabled={status === 'on' || pending != null}
        onClick={click('on')}
      >
        {pending === 'on' ? <RotateCw className="animate-spin" /> : <Power />}
        On
      </Button>
      <Button
        size={size}
        variant="destructive"
        disabled={status === 'off' || pending != null}
        onClick={click('off')}
      >
        {pending === 'off' ? <RotateCw className="animate-spin" /> : <PowerOff />}
        Off
      </Button>
    </div>
  )
}
