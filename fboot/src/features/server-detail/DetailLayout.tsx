import type { ReactNode } from 'react'
import { NavLink } from 'react-router-dom'
import { cn } from '@/lib/utils'
import { Separator } from '@/components/ui/separator'
import { RecentServersSidebar } from './RecentServersSidebar'

export interface DetailSection {
  key: string
  label: string
  icon: ReactNode
}

export interface DetailLayoutProps {
  title: string
  subtitle?: string
  recent: string[]
  activeId: string
  sections: DetailSection[]
  powerSlot?: ReactNode
  children: ReactNode
}

export function DetailLayout({
  title,
  subtitle,
  recent,
  activeId,
  sections,
  powerSlot,
  children,
}: DetailLayoutProps) {
  return (
    <div className="grid gap-6 lg:grid-cols-[16rem_1fr]">
      <aside className="flex flex-col gap-4">
        <RecentServersSidebar recent={recent} activeId={activeId} />
        <Separator />
        <div className="flex flex-col gap-1">
          <div className="px-2 pb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            {title}
          </div>
          {subtitle && <div className="px-2 pb-1 font-mono text-xs text-muted-foreground">{subtitle}</div>}
          {powerSlot && <div className="px-2 pb-2 pt-1">{powerSlot}</div>}
          {sections.map((section) => (
            <NavLink
              key={section.key}
              to={section.key === 'info' ? `/servers/${activeId}` : `/servers/${activeId}/${section.key}`}
              end={section.key === 'info'}
              className={({ isActive }) =>
                cn(
                  'flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm transition-colors hover:bg-accent',
                  isActive && 'bg-accent font-medium',
                )
              }
            >
              {section.icon}
              {section.label}
            </NavLink>
          ))}
        </div>
      </aside>
      <div className="min-w-0">{children}</div>
    </div>
  )
}
