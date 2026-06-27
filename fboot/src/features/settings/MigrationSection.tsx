import { useRef, useState } from 'react'
import { AlertTriangle, Download, Loader2, Upload } from 'lucide-react'
import { api } from '@/api'
import { Button } from '@/components/ui/button'
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { useAppSelector } from '@/store/hooks'
import { formatBytes } from '@/lib/format'

export function MigrationSection() {
  const wsStatus = useAppSelector((s) => s.ui.wsStatus)
  const inputRef = useRef<HTMLInputElement>(null)

  const [pending, setPending] = useState<File | null>(null)
  const [importing, setImporting] = useState(false)
  const [restarting, setRestarting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const pickFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    e.target.value = ''
    if (!file) return
    setError(null)
    setPending(file)
  }

  const confirmRestore = async () => {
    if (!pending) return
    setImporting(true)
    setError(null)
    try {
      const { restarting: willRestart } = await api.migration.import(pending)
      setPending(null)
      if (willRestart) setRestarting(true)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to restore backup')
    } finally {
      setImporting(false)
    }
  }

  // Once the server is back, the websocket flips to 'open' again.
  const reconnected = restarting && wsStatus === 'open'

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">Migration</CardTitle>
        <CardDescription>Download a full backup, or restore from one.</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-5">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div className="space-y-0.5">
            <p className="text-sm font-medium">Download backup</p>
            <p className="text-sm text-muted-foreground">
              A full <code className="font-mono text-xs">tar.gz</code> of the database and blobs.
            </p>
          </div>
          <Button asChild variant="outline" className="shrink-0">
            <a href={api.migration.exportUrl()} download>
              <Download className="size-4" /> Download backup
            </a>
          </Button>
        </div>

        <div className="flex flex-col gap-3 border-t pt-5 sm:flex-row sm:items-start sm:justify-between">
          <div className="space-y-0.5">
            <p className="text-sm font-medium">Restore from file</p>
            <p className="text-sm text-muted-foreground">
              Replace all current state with a previously downloaded backup.
            </p>
          </div>
          <input
            ref={inputRef}
            type="file"
            accept=".tar.gz,.gz,application/gzip"
            className="hidden"
            onChange={pickFile}
          />
          <Button
            variant="outline"
            className="shrink-0"
            onClick={() => inputRef.current?.click()}
            disabled={importing || restarting}
          >
            <Upload className="size-4" /> Restore from file
          </Button>
        </div>

        {error && <p className="text-sm text-destructive">{error}</p>}

        {restarting && (
          <div className="flex items-center gap-2 rounded-md border bg-muted/40 px-3 py-2.5 text-sm">
            {reconnected ? (
              <span className="text-success-foreground">
                Restore complete — server reconnected.
              </span>
            ) : (
              <>
                <Loader2 className="size-4 animate-spin text-muted-foreground" />
                <span className="text-muted-foreground">Server is restarting, reconnecting…</span>
              </>
            )}
          </div>
        )}
      </CardContent>

      <Dialog open={pending !== null} onOpenChange={(open) => !open && !importing && setPending(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertTriangle className="size-5 text-warning" /> Restore from backup?
            </DialogTitle>
            <DialogDescription asChild>
              <div className="space-y-2">
                <p>
                  Restoring overwrites everything. Click <strong>Download backup</strong> first if
                  you haven&apos;t.
                </p>
                <p>
                  A safety dump (<code className="font-mono text-xs">migration.bak.tar.gz</code>) is
                  saved automatically, and <strong>the server will restart</strong>.
                </p>
              </div>
            </DialogDescription>
          </DialogHeader>
          {pending && (
            <p className="rounded-md border bg-muted/40 px-3 py-2 text-sm">
              <span className="font-mono">{pending.name}</span>
              <span className="ml-2 text-muted-foreground">{formatBytes(pending.size)}</span>
            </p>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setPending(null)} disabled={importing}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={confirmRestore} disabled={importing}>
              {importing ? 'Restoring…' : 'Restore & restart'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  )
}
