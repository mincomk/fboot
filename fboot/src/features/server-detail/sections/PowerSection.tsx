import { Power, PowerOff, RotateCw, RefreshCw } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { StatusBadge } from '@/components/shared/StatusBadge'
import { useAppDispatch } from '@/store/hooks'
import { powerAction } from '@/store/slices/servers'
import type { PowerAction } from '@/api'
import type { ServerView } from '@/hooks/useServers'

const ACTIONS: { action: PowerAction; label: string; icon: typeof Power; variant: 'success' | 'destructive' | 'secondary' | 'outline' }[] = [
  { action: 'on', label: 'Power On', icon: Power, variant: 'success' },
  { action: 'off', label: 'Power Off', icon: PowerOff, variant: 'destructive' },
  { action: 'cycle', label: 'Power Cycle', icon: RotateCw, variant: 'secondary' },
  { action: 'status', label: 'Refresh Status', icon: RefreshCw, variant: 'outline' },
]

export function PowerSection({ view }: { view: ServerView }) {
  const dispatch = useAppDispatch()
  const power = view.stats?.power_status ?? 'unknown'
  const id = view.server.id

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between">
        <CardTitle>Power Control</CardTitle>
        <StatusBadge status={power} />
      </CardHeader>
      <CardContent className="flex flex-wrap gap-3">
        {ACTIONS.map(({ action, label, icon: Icon, variant }) => (
          <Button
            key={action}
            variant={variant}
            onClick={() => dispatch(powerAction({ id, action }))}
          >
            <Icon /> {label}
          </Button>
        ))}
      </CardContent>
    </Card>
  )
}
