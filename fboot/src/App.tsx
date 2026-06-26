import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { TooltipProvider } from '@/components/ui/tooltip'
import { AppShell } from '@/components/shared/AppShell'
import { DashboardPage } from '@/pages/DashboardPage'
import { ServerDetailPage } from '@/pages/ServerDetailPage'
import { ScanPage } from '@/pages/ScanPage'
import { BootablesPage } from '@/pages/BootablesPage'

export default function App() {
  return (
    <BrowserRouter>
      <TooltipProvider delayDuration={200}>
        <AppShell>
          <Routes>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/servers/:id" element={<ServerDetailPage />} />
            <Route path="/servers/:id/:section" element={<ServerDetailPage />} />
            <Route path="/bootables" element={<BootablesPage />} />
            <Route path="/scan" element={<ScanPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </AppShell>
      </TooltipProvider>
    </BrowserRouter>
  )
}
