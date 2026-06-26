import { RotateCw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export interface RefreshButtonProps {
  onClick: () => void
  spinning?: boolean
}

export function RefreshButton({ onClick, spinning }: RefreshButtonProps) {
  return (
    <Button
      type="button"
      variant="outline"
      size="icon"
      onClick={onClick}
      disabled={spinning}
      aria-label="Refresh"
    >
      <RotateCw className={cn(spinning && 'animate-spin')} />
    </Button>
  )
}
