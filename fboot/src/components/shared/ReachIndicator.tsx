import { cn } from '@/lib/utils'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'

export interface ReachIndicatorProps {
  label: string
  active: boolean
  inactiveLabel?: string
}

export function ReachIndicator({ label, active, inactiveLabel }: ReachIndicatorProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
          <span
            className={cn(
              'size-2 rounded-full',
              active ? 'bg-success' : 'bg-muted-foreground/40',
            )}
          />
          {label}
        </span>
      </TooltipTrigger>
      <TooltipContent>{active ? `${label} reachable` : (inactiveLabel ?? `${label} unreachable`)}</TooltipContent>
    </Tooltip>
  )
}
