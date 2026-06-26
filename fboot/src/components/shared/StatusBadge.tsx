import { Power } from 'lucide-react'
import { Badge } from '@/components/ui/badge'
import type { PowerStatus } from '@/api'

const LABELS: Record<PowerStatus, string> = {
  on: 'On',
  off: 'Off',
  unknown: 'Unknown',
}

const VARIANTS: Record<PowerStatus, 'success' | 'muted' | 'secondary'> = {
  on: 'success',
  off: 'muted',
  unknown: 'secondary',
}

export function StatusBadge({ status }: { status: PowerStatus }) {
  return (
    <Badge variant={VARIANTS[status]}>
      <Power className="size-3" />
      {LABELS[status]}
    </Badge>
  )
}
