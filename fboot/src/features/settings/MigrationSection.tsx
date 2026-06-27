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
import { Progress } from '@/components/ui/progress'
import { useAppSelector } from '@/store/hooks'
import { formatBytes } from '@/lib/format'

function triggerBlobDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  a.remove()
  URL.revokeObjectURL(url)
}

export function MigrationSection() {
  const wsStatus = useAppSelector((s) => s.ui.wsStatus)
  const inputRef = useRef<HTMLInputElement>(null)

  const [pending, setPending] = useState<File | null>(null)
  const [importing, setImporting] = useState(false)
  const [uploadPct, setUploadPct] = useState<number | null>(null)
  const [restarting, setRestarting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [downloading, setDownloading] = useState(false)
  const [downloadPct, setDownloadPct] = useState<number | null>(null)
  const [downloadError, setDownloadError] = useState<string | null>(null)

  const handleDownload = async () => {
    setDownloading(true)
    setDownloadPct(null)
    setDownloadError(null)
    try {
      const { blob, filename } = await api.migration.download((loaded, total) =>
        setDownloadPct(total ? Math.round((loaded / total) * 100) : null),
      )
      triggerBlobDownload(blob, filename)
    } catch (e) {
      setDownloadError(e instanceof Error ? e.message : 'Failed to download backup')
    } finally {
      setDownloading(false)
      setDownloadPct(null)
    }
  }

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
    setUploadPct(null)
    setError(null)
    try {
      const { restarting: willRestart } = await api.migration.import(pending, (loaded, total) =>
        setUploadPct(total ? Math.round((loaded / total) * 100) : null),
      )
      setPending(null)
      if (willRestart) setRestarting(true)
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to restore backup')
    } finally {
      setImporting(false)
      setUploadPct(null)
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
        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div className="space-y-0.5">
              <p className="text-sm font-medium">Download backup</p>
              <p className="text-sm text-muted-foreground">
                A full <code className="font-mono text-xs">tar.gz</code> of the database and blobs.
              </p>
            </div>
            <Button
              variant="outline"
              className="shrink-0"
              onClick={handleDownload}
              disabled={downloading}
            >
              {downloading ? (
                <Loader2 className="size-4 animate-spin" />
              ) : (
                <Download className="size-4" />
              )}
              {downloading ? 'Downloading…' : 'Download backup'}
            </Button>
          </div>
          {downloading &&
            (downloadPct === null ? (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" /> Preparing backup…
              </div>
            ) : (
              <div className="flex flex-col gap-1.5">
                <Progress value={downloadPct} />
                <p className="text-xs text-muted-foreground">{downloadPct}%</p>
              </div>
            ))}
          {downloadError && <p className="text-sm text-destructive">{downloadError}</p>}
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
          {importing && (
            <div className="flex flex-col gap-1.5">
              <Progress value={uploadPct ?? 0} />
              <p className="text-xs text-muted-foreground">
                {uploadPct === null ? 'Uploading…' : `Uploading… ${uploadPct}%`}
              </p>
            </div>
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
