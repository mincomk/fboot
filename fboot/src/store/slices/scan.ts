import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import type { ScanResult } from '@/api'

export interface ScanState {
  running: boolean
  results: ScanResult[]
  scanned: number
  total: number
}

const initialState: ScanState = {
  running: false,
  results: [],
  scanned: 0,
  total: 0,
}

const slice = createSlice({
  name: 'scan',
  initialState,
  reducers: {
    scanStarted(state) {
      state.running = true
      state.results = []
      state.scanned = 0
      state.total = 0
    },
    scanResult(state, action: PayloadAction<ScanResult>) {
      const existing = state.results.findIndex((r) => r.ip === action.payload.ip)
      if (existing >= 0) state.results[existing] = action.payload
      else state.results.push(action.payload)
    },
    scanProgress(state, action: PayloadAction<{ scanned: number; total: number }>) {
      state.scanned = action.payload.scanned
      state.total = action.payload.total
    },
    scanDone(state) {
      state.running = false
    },
  },
})

export const { scanStarted, scanResult, scanProgress, scanDone } = slice.actions
export default slice.reducer
