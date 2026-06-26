import { useEffect } from 'react'
import { useAppDispatch } from '@/store/hooks'
import { wsConnect, wsDisconnect } from '@/store/middleware/wsMiddleware'

export function useServerEvents() {
  const dispatch = useAppDispatch()
  useEffect(() => {
    dispatch(wsConnect)
    return () => {
      dispatch(wsDisconnect)
    }
  }, [dispatch])
}
