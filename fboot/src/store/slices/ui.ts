import { createSlice, type PayloadAction } from '@reduxjs/toolkit'
import { applyTheme, getStoredTheme, type Theme } from '@/lib/theme'

export type ViewMode = 'card' | 'list'
export type WsStatus = 'connecting' | 'open' | 'closed'

export interface UiState {
  theme: Theme
  viewMode: ViewMode
  selectedServerIds: string[]
  wsStatus: WsStatus
}

const initialState: UiState = {
  theme: getStoredTheme(),
  viewMode: 'card',
  selectedServerIds: [],
  wsStatus: 'connecting',
}

const slice = createSlice({
  name: 'ui',
  initialState,
  reducers: {
    setTheme(state, action: PayloadAction<Theme>) {
      state.theme = action.payload
      applyTheme(action.payload)
    },
    toggleTheme(state) {
      state.theme = state.theme === 'dark' ? 'light' : 'dark'
      applyTheme(state.theme)
    },
    setViewMode(state, action: PayloadAction<ViewMode>) {
      state.viewMode = action.payload
    },
    toggleServerSelection(state, action: PayloadAction<string>) {
      const id = action.payload
      if (state.selectedServerIds.includes(id))
        state.selectedServerIds = state.selectedServerIds.filter((s) => s !== id)
      else state.selectedServerIds.push(id)
    },
    setSelection(state, action: PayloadAction<string[]>) {
      state.selectedServerIds = action.payload
    },
    clearSelection(state) {
      state.selectedServerIds = []
    },
    setWsStatus(state, action: PayloadAction<WsStatus>) {
      state.wsStatus = action.payload
    },
  },
})

export const {
  setTheme,
  toggleTheme,
  setViewMode,
  toggleServerSelection,
  setSelection,
  clearSelection,
  setWsStatus,
} = slice.actions
export default slice.reducer
