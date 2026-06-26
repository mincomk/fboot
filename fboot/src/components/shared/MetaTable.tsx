export interface MetaRow {
  label: string
  value: React.ReactNode
}

export function MetaTable({ rows }: { rows: MetaRow[] }) {
  return (
    <dl className="grid grid-cols-[max-content_1fr] gap-x-6 gap-y-2 text-sm">
      {rows.map((row) => (
        <div key={row.label} className="contents">
          <dt className="text-muted-foreground">{row.label}</dt>
          <dd className="break-all font-medium">{row.value ?? '—'}</dd>
        </div>
      ))}
    </dl>
  )
}
