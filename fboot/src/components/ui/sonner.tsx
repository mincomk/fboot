import { Toaster as Sonner } from 'sonner'
import { useAppSelector } from '@/store/hooks'

export function Toaster(props: React.ComponentProps<typeof Sonner>) {
  const theme = useAppSelector((s) => s.ui.theme)
  return <Sonner theme={theme} richColors position="bottom-right" {...props} />
}
