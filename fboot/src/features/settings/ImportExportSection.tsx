import { useRef, useState } from 'react'
import { AlertTriangle, Download, FileJson, Upload } from 'lucide-react'
import { api } from '@/api'
import type {
  ImportConflict,
  ImportMode,
  ImportResult,
  ServerExportOptions,
  ServerRecord,
} from '@/api/types'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
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
import { ConflictResolutionDialog } from './ConflictResolutionDialog'

type ExportField = 'status' | 'config' | 'mac' | 'ip'

const EXPORT_FIELDS: { key: ExportField; label: string; hint: string }[] = [
  { key: 'status', label: 'Status', hint: 'Power state, draw, temperature' },
  { key: 'config', label: 'Config', hint: 'Boot settings and cmdline' },
  { key: 'mac', label: 'MAC', hint: 'Primary and IPMI MAC addresses' },
  { key: 'ip', label: 'IP', hint: 'Primary and IPMI IP addresses' },
]

function summarize(result: ImportResult): string {
  const parts: string[] = []
  if (result.imported != null) parts.push(`${result.imported} imported`)
  if (result.overwritten != null) parts.push(`${result.overwritten} overwritten`)
  if (result.skipped != null) parts.push(`${result.skipped} skipped`)
  return parts.length ? parts.join(', ') : 'No changes applied'
}

export function ImportExportSection() {
  const fileInputRef = useRef<HTMLInputElement>(null)

  // Export dialog state
  const [exportOpen, setExportOpen] = useState(false)
  const [opts, setOpts] = useState<ServerExportOptions>({
    status: true,
    config: true,
    mac: true,
    ip: true,
    pretty: true,
  })
  const [exporting, setExporting] = useState(false)
  const [exportError, setExportError] = useState<string | null>(null)

  // Import mode dialog state
  const [pending, setPending] = useState<ServerRecord[] | null>(null)
  const [mode, setMode] = useState<ImportMode>('append')
  const [importing, setImporting] = useState(false)
  const [importError, setImportError] = useState<string | null>(null)

  // Conflict + result state
  const [conflicts, setConflicts] = useState<ImportConflict[] | null>(null)
  const [conflictServers, setConflictServers] = useState<ServerRecord[]>([])
  const [conflictMode, setConflictMode] = useState<ImportMode>('append')
  const [summary, setSummary] = useState<string | null>(null)

  const setOpt = (key: keyof ServerExportOptions, value: boolean) =>
    setOpts((prev) => ({ ...prev, [key]: value }))

  const runExport = async () => {
    setExportError(null)
    setExporting(true)
    try {
      const records = await api.serversIo.export(opts)
      const text = JSON.stringify(records, null, opts.pretty ? 2 : 0)
      const blob = new Blob([text], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = 'fboot-servers.json'
      document.body.appendChild(a)
      a.click()
      a.remove()
      URL.revokeObjectURL(url)
      setExportOpen(false)
    } catch (e) {
      setExportError(e instanceof Error ? e.message : 'Export failed')
    } finally {
      setExporting(false)
    }
  }

  const onFilePicked = async (e: React.ChangeEvent<HTMLInputElement>) => {
    setImportError(null)
    setSummary(null)
    const file = e.target.files?.[0]
    e.target.value = '' // allow re-selecting the same file
    if (!file) return
    try {
      const parsed: unknown = JSON.parse(await file.text())
      if (!Array.isArray(parsed)) {
        setImportError('Invalid file: expected a JSON array of server records.')
        return
      }
      setMode('append')
      setPending(parsed as ServerRecord[])
    } catch {
      setImportError('Invalid file: could not parse JSON.')
    }
  }

  const runImport = async () => {
    if (!pending) return
    setImportError(null)
    setImporting(true)
    try {
      const result = await api.serversIo.import({ mode, servers: pending })
      if (result.conflicts && result.conflicts.length > 0) {
        setConflicts(result.conflicts)
        setConflictServers(pending)
        setConflictMode(mode)
      } else {
        setSummary(summarize(result))
      }
      setPending(null)
    } catch (e) {
      setImportError(e instanceof Error ? e.message : 'Import failed')
    } finally {
      setImporting(false)
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>Import / Export Servers</CardTitle>
        <CardDescription>Move your server inventory in and out as JSON.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" onClick={() => setExportOpen(true)}>
            <Download /> Export
          </Button>
          <Button variant="outline" onClick={() => fileInputRef.current?.click()}>
            <Upload /> Import
          </Button>
          <input
            ref={fileInputRef}
            type="file"
            accept=".json,application/json"
            className="hidden"
            onChange={onFilePicked}
          />
        </div>

        {summary && (
          <div className="flex items-center gap-2 rounded-md border border-success/40 bg-success/10 px-3 py-2 text-sm">
            <FileJson className="size-4 text-success" />
            <span>Import complete — {summary}.</span>
          </div>
        )}
        {importError && <p className="text-sm text-destructive">{importError}</p>}
      </CardContent>

      {/* Export options dialog */}
      <Dialog open={exportOpen} onOpenChange={setExportOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Export servers</DialogTitle>
            <DialogDescription>Choose which data to include in the JSON file.</DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-3">
            {EXPORT_FIELDS.map((f) => (
              <label key={f.key} className="flex items-start gap-3">
                <Checkbox
                  checked={opts[f.key]}
                  onCheckedChange={(v) => setOpt(f.key, v === true)}
                  className="mt-0.5"
                />
                <div className="flex flex-col gap-0.5">
                  <span className="text-sm font-medium leading-none">{f.label}</span>
                  <span className="text-xs text-muted-foreground">{f.hint}</span>
                </div>
              </label>
            ))}
            <Separator />
            <label className="flex items-center gap-3">
              <Checkbox
                checked={opts.pretty}
                onCheckedChange={(v) => setOpt('pretty', v === true)}
              />
              <span className="text-sm font-medium leading-none">Pretty print JSON</span>
            </label>
          </div>
          {exportError && <p className="text-sm text-destructive">{exportError}</p>}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setExportOpen(false)}>
              Cancel
            </Button>
            <Button type="button" onClick={runExport} disabled={exporting}>
              {exporting ? 'Exporting…' : 'Download JSON'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Import mode dialog */}
      <Dialog open={pending !== null} onOpenChange={(o) => !o && setPending(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Import servers</DialogTitle>
            <DialogDescription>
              {pending?.length ?? 0} record{(pending?.length ?? 0) === 1 ? '' : 's'} loaded. Pick
              how to apply them.
            </DialogDescription>
          </DialogHeader>
          <div className="flex flex-col gap-2">
            <button
              type="button"
              onClick={() => setMode('append')}
              className={cn(
                'rounded-lg border p-3 text-left transition-colors',
                mode === 'append' ? 'border-primary bg-primary/5' : 'border-input hover:bg-muted',
              )}
            >
              <span className="text-sm font-medium">Append</span>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Add new servers, resolving any conflicts with existing records.
              </p>
            </button>
            <button
              type="button"
              onClick={() => setMode('override')}
              className={cn(
                'rounded-lg border p-3 text-left transition-colors',
                mode === 'override'
                  ? 'border-destructive bg-destructive/5'
                  : 'border-input hover:bg-muted',
              )}
            >
              <span className="flex items-center gap-1.5 text-sm font-medium">
                <AlertTriangle className="size-4 text-destructive" /> Override
              </span>
              <p className="mt-0.5 text-xs text-muted-foreground">
                Replaces ALL existing servers. A <code>servers.bak.json</code> safety dump is saved
                automatically.
              </p>
            </button>
          </div>
          {importError && <p className="text-sm text-destructive">{importError}</p>}
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => setPending(null)}>
              Cancel
            </Button>
            <Button
              type="button"
              variant={mode === 'override' ? 'destructive' : 'default'}
              onClick={runImport}
              disabled={importing}
            >
              {importing ? 'Importing…' : mode === 'override' ? 'Override all' : 'Import'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Conflict resolution */}
      {conflicts && (
        <ConflictResolutionDialog
          open={conflicts !== null}
          onOpenChange={(o) => !o && setConflicts(null)}
          conflicts={conflicts}
          servers={conflictServers}
          mode={conflictMode}
          onResolved={(result) => {
            setSummary(summarize(result))
            setConflicts(null)
          }}
        />
      )}
    </Card>
  )
}
