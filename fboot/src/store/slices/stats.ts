import { createAsyncThunk, createSlice, type PayloadAction } from '@reduxjs/toolkit'
import { api } from '@/api'
import type { StatsSample } from '@/api'
import { powerAction } from './servers'

export interface StatsState {
  latest: Record<string, StatsSample>
  loading: boolean
}

const initialState: StatsState = {
  latest: {},
  loading: false,
}

export const fetchStats = createAsyncThunk('stats/fetch', () => api.stats.latest())

const slice = createSlice({
  name: 'stats',
  initialState,
  reducers: {
    statsUpdated(state, action: PayloadAction<StatsSample>) {
      state.latest[action.payload.server_id] = action.payload
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchStats.pending, (state) => {
        state.loading = true
      })
      .addCase(fetchStats.fulfilled, (state, action) => {
        state.loading = false
        for (const sample of action.payload) state.latest[sample.server_id] = sample
      })
      .addCase(fetchStats.rejected, (state) => {
        state.loading = false
      })
      .addCase(powerAction.fulfilled, (state, action) => {
        const sample = state.latest[action.payload.id]
        if (sample && (action.payload.power === 'on' || action.payload.power === 'off')) {
          sample.power_status = action.payload.power
        }
      })
  },
})

export const { statsUpdated } = slice.actions
export default slice.reducer
