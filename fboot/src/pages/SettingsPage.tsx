import { MigrationSection } from '@/features/settings/MigrationSection'
import { ImportExportSection } from '@/features/settings/ImportExportSection'
import { CacheSection } from '@/features/settings/CacheSection'

export function SettingsPage() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="text-sm text-muted-foreground">
          Back up and restore the server, import/export your inventory, and manage cached state.
        </p>
      </div>
      <MigrationSection />
      <ImportExportSection />
      <CacheSection />
    </div>
  )
}
