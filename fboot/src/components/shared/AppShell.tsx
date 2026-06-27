import type { ReactNode } from 'react'
import { Link, NavLink } from 'react-router-dom'
import { HardDriveDownload, LayoutDashboard, Radar, Settings } from 'lucide-react'
import { cn } from '@/lib/utils'
import { ThemeToggle } from './ThemeToggle'
import { useAppSelector } from '@/store/hooks'
import { useServerEvents } from '@/hooks/useServerEvents'

const NAV = [
  { to: '/', label: 'Dashboard', icon: LayoutDashboard, end: true },
  { to: '/bootables', label: 'Bootables', icon: HardDriveDownload, end: false },
  { to: '/scan', label: 'Scan', icon: Radar, end: false },
  { to: '/settings', label: 'Settings', icon: Settings, end: false },
]

const WS_LABEL = {
  connecting: 'Connecting',
  open: 'Live',
  closed: 'Offline',
} as const

const WS_DOT = {
  connecting: 'bg-warning',
  open: 'bg-success',
  closed: 'bg-muted-foreground/50',
} as const

export function AppShell({ children }: { children: ReactNode }) {
  useServerEvents()
  const wsStatus = useAppSelector((s) => s.ui.wsStatus)

  return (
    <div className="min-h-screen bg-background">
      <header className="sticky top-0 z-20 border-b bg-background/80 backdrop-blur">
        <div className="mx-auto flex h-14 max-w-7xl items-center gap-6 px-4">
          <Link
            to="/"
            className="flex items-center gap-2 font-semibold transition-opacity hover:opacity-80"
          >
            <HardDriveDownload className="size-5 text-primary" />
            fboot
          </Link>
          <nav className="flex items-center gap-1">
            {NAV.map(({ to, label, icon: Icon, end }) => (
              <NavLink
                key={to}
                to={to}
                end={end}
                className={({ isActive }) =>
                  cn(
                    'flex items-center gap-2 rounded-md px-3 py-1.5 text-sm transition-colors hover:bg-accent',
                    isActive && 'bg-accent font-medium',
                  )
                }
              >
                <Icon className="size-4" />
                {label}
              </NavLink>
            ))}
          </nav>
          <div className="ml-auto flex items-center gap-3">
            <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <span className={cn('size-2 rounded-full', WS_DOT[wsStatus])} />
              {WS_LABEL[wsStatus]}
            </span>
            <ThemeToggle />
          </div>
        </div>
      </header>
      <main className="mx-auto max-w-7xl px-4 py-6">{children}</main>
    </div>
  )
}
