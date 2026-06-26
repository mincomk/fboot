import { LayoutGrid, List } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import type { ViewMode } from '@/store/slices/ui'

export interface ViewToggleProps {
  value: ViewMode
  onChange: (mode: ViewMode) => void
}

export function ViewToggle({ value, onChange }: ViewToggleProps) {
  return (
    <div className="inline-flex items-center rounded-md border p-0.5">
      <Button
        size="icon"
        variant="ghost"
        className={cn('size-7', value === 'card' && 'bg-accent text-accent-foreground')}
        onClick={() => onChange('card')}
        aria-label="Card view"
      >
        <LayoutGrid />
      </Button>
      <Button
        size="icon"
        variant="ghost"
        className={cn('size-7', value === 'list' && 'bg-accent text-accent-foreground')}
        onClick={() => onChange('list')}
        aria-label="List view"
      >
        <List />
      </Button>
    </div>
  )
}
