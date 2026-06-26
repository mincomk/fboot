import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { ConsoleStatus } from '@/api'

export interface ConsoleState {
  byServer: Record<string, ConsoleStatus>
}

const initialState: ConsoleState = {
  byServer: {},
}

const slice = createSlice({
  name: 'console',
  initialState,
  reducers: {
    consoleStatusChanged(
      state,
      action: PayloadAction<{ server_id: string; status: ConsoleStatus }>,
    ) {
      state.byServer[action.payload.server_id] = action.payload.status
    },
  },
})

export const { consoleStatusChanged } = slice.actions
export default slice.reducer
