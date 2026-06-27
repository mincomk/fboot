import { useMemo, useState } from 'react'
import { AlertTriangle, ArrowRightLeft } from 'lucide-react'
import { api } from '@/api'
import type {
  ConflictChoice,
  ImportConflict,
  ImportMode,
  ImportResult,
  ServerRecord,
} from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Separator } from '@/components/ui/separator'
import { cn } from '@/lib/utils'

interface ConflictResolutionDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  conflicts: ImportConflict[]
  servers: ServerRecord[]
  mode: ImportMode
  onResolved: (result: ImportResult) => void
}

const FIELDS: { key: keyof ServerRecord; label: string }[] = [
  { key: 'friendly_name', label: 'Name' },
  { key: 'hostname', label: 'Host' },
  { key: 'primary_mac', label: 'MAC' },
  { key: 'ipmi_mac', label: 'IPMI MAC' },
]

function FieldList({ record }: { record: ServerRecord }) {
  return (
    <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
      {FIELDS.map((f) => (
        <div key={f.key} className="contents">
          <dt className="text-muted-foreground">{f.label}</dt>
          <dd className="truncate font-mono text-foreground">{String(record[f.key] ?? '—')}</dd>
        </div>
      ))}
    </dl>
  )
}

function ChoiceButton({
  active,
  onClick,
  children,
  variant,
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
  variant: 'original' | 'new'
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'flex-1 rounded-md border px-3 py-1.5 text-xs font-medium transition-colors',
        active
          ? variant === 'original'
            ? 'border-primary bg-primary/10 text-foreground'
            : 'border-warning bg-warning/10 text-foreground'
          : 'border-input text-muted-foreground hover:bg-muted',
      )}
    >
      {children}
    </button>
  )
}

export function ConflictResolutionDialog({
  open,
  onOpenChange,
  conflicts,
  servers,
  mode,
  onResolved,
}: ConflictResolutionDialogProps) {
  // Default every conflict to the safe choice: keep the existing record.
  const initial = useMemo<Record<string, ConflictChoice>>(
    () => Object.fromEntries(conflicts.map((c) => [c.key, 'original'])),
    [conflicts],
  )
  const [choices, setChoices] = useState<Record<string, ConflictChoice>>(initial)
  const [applying, setApplying] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const setAll = (choice: ConflictChoice) =>
    setChoices(Object.fromEntries(conflicts.map((c) => [c.key, choice])))

  const setOne = (key: string, choice: ConflictChoice) =>
    setChoices((prev) => ({ ...prev, [key]: choice }))

  const apply = async () => {
    setError(null)
    setApplying(true)
    try {
      const result = await api.serversIo.import({ mode, servers, resolutions: choices })
      onResolved(result)
      onOpenChange(false)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to apply resolutions')
    } finally {
      setApplying(false)
    }
  }

  const newCount = Object.values(choices).filter((c) => c === 'new').length

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertTriangle className="size-5 text-warning" /> Resolve import conflicts
          </DialogTitle>
          <DialogDescription>
            {conflicts.length} incoming server{conflicts.length === 1 ? '' : 's'} match existing
            records. Choose which version to keep for each.
          </DialogDescription>
        </DialogHeader>

        <div className="flex items-center justify-between gap-2">
          <span className="text-xs text-muted-foreground">
            {newCount} of {conflicts.length} set to overwrite
          </span>
          <div className="flex gap-2">
            <Button type="button" variant="outline" size="sm" onClick={() => setAll('original')}>
              All Original
            </Button>
            <Button type="button" variant="outline" size="sm" onClick={() => setAll('new')}>
              All New
            </Button>
          </div>
        </div>

        <div className="max-h-[50vh] space-y-3 overflow-y-auto pr-1">
          {conflicts.map((c) => {
            const choice = choices[c.key] ?? 'original'
            return (
              <div key={c.key} className="rounded-lg border bg-card p-3">
                <div className="mb-2 flex items-center gap-2">
                  <Badge variant="outline" className="font-mono">
                    {c.key}
                  </Badge>
                  <ArrowRightLeft className="size-3.5 text-muted-foreground" />
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div
                    className={cn(
                      'rounded-md border p-2',
                      choice === 'original' && 'ring-1 ring-primary',
                    )}
                  >
                    <p className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                      Existing
                    </p>
                    <FieldList record={c.existing} />
                  </div>
                  <div
                    className={cn(
                      'rounded-md border p-2',
                      choice === 'new' && 'ring-1 ring-warning',
                    )}
                  >
                    <p className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                      Incoming
                    </p>
                    <FieldList record={c.incoming} />
                  </div>
                </div>
                <div className="mt-2 flex gap-2">
                  <ChoiceButton
                    variant="original"
                    active={choice === 'original'}
                    onClick={() => setOne(c.key, 'original')}
                  >
                    Take Original
                  </ChoiceButton>
                  <ChoiceButton
                    variant="new"
                    active={choice === 'new'}
                    onClick={() => setOne(c.key, 'new')}
                  >
                    Take New
                  </ChoiceButton>
                </div>
              </div>
            )
          })}
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <Separator />
        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" onClick={apply} disabled={applying}>
            {applying ? 'Applying…' : 'Apply resolutions'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
