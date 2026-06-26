import { configureStore } from '@reduxjs/toolkit'
import servers from './slices/servers'
import bootables from './slices/bootables'
import stats from './slices/stats'
import scan from './slices/scan'
import ui from './slices/ui'
import console from './slices/console'
import { wsMiddleware } from './middleware/wsMiddleware'

export const store = configureStore({
  reducer: { servers, bootables, stats, scan, ui, console },
  middleware: (getDefault) => getDefault().concat(wsMiddleware),
})

export type RootState = ReturnType<typeof store.getState>
export type AppDispatch = typeof store.dispatch
