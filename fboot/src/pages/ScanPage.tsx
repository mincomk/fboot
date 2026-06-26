import { ScanPanel } from '@/features/scan/ScanPanel'

export function ScanPage() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Network Scan</h1>
        <p className="text-sm text-muted-foreground">Discover hosts on the network by IPMI, SSH, or custom port.</p>
      </div>
      <ScanPanel />
    </div>
  )
}
