import { Moon, Sun } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useAppDispatch, useAppSelector } from '@/store/hooks'
import { toggleTheme } from '@/store/slices/ui'

export function ThemeToggle() {
  const theme = useAppSelector((s) => s.ui.theme)
  const dispatch = useAppDispatch()
  return (
    <Button
      size="icon"
      variant="ghost"
      onClick={() => dispatch(toggleTheme())}
      aria-label="Toggle theme"
    >
      {theme === 'dark' ? <Sun /> : <Moon />}
    </Button>
  )
}
