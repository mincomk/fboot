import { createAsyncThunk, createSlice } from '@reduxjs/toolkit'
import { api } from '@/api'
import type { Bootable, BootableRole, BootDefaults, NewBootable } from '@/api'

export interface BootablesState {
  items: Bootable[]
  defaults: BootDefaults
  loading: boolean
  error: string | null
}

const initialState: BootablesState = {
  items: [],
  defaults: {},
  loading: false,
  error: null,
}

export const fetchBootables = createAsyncThunk('bootables/fetch', () => api.bootables.list())

export const createBootable = createAsyncThunk('bootables/create', (body: NewBootable) =>
  api.bootables.create(body),
)

export const updateBootable = createAsyncThunk(
  'bootables/update',
  (args: { id: string; patch: Partial<NewBootable> }) =>
    api.bootables.update(args.id, args.patch),
)

export const deleteBootable = createAsyncThunk('bootables/delete', async (id: string) => {
  await api.bootables.remove(id)
  return id
})

export const uploadBootableFile = createAsyncThunk(
  'bootables/upload',
  (args: { id: string; role: BootableRole; file: File }) =>
    api.bootables.upload(args.id, args.role, args.file),
)

export const fetchBootDefaults = createAsyncThunk('bootables/fetchDefaults', () =>
  api.bootDefaults.get(),
)

export const updateBootDefaults = createAsyncThunk(
  'bootables/updateDefaults',
  (body: BootDefaults) => api.bootDefaults.update(body),
)

const slice = createSlice({
  name: 'bootables',
  initialState,
  reducers: {},
  extraReducers: (builder) => {
    builder
      .addCase(fetchBootables.pending, (state) => {
        state.loading = true
        state.error = null
      })
      .addCase(fetchBootables.fulfilled, (state, action) => {
        state.loading = false
        state.items = action.payload
      })
      .addCase(fetchBootables.rejected, (state, action) => {
        state.loading = false
        state.error = action.error.message ?? 'Failed to load bootables'
      })
      .addCase(createBootable.fulfilled, (state, action) => {
        state.items.push(action.payload)
      })
      .addCase(updateBootable.fulfilled, (state, action) => {
        const idx = state.items.findIndex((b) => b.id === action.payload.id)
        if (idx >= 0) state.items[idx] = action.payload
      })
      .addCase(deleteBootable.fulfilled, (state, action) => {
        state.items = state.items.filter((b) => b.id !== action.payload)
      })
      .addCase(uploadBootableFile.fulfilled, (state, action) => {
        const idx = state.items.findIndex((b) => b.id === action.payload.id)
        if (idx >= 0) state.items[idx] = action.payload
      })
      .addCase(fetchBootDefaults.fulfilled, (state, action) => {
        state.defaults = action.payload
      })
      .addCase(updateBootDefaults.fulfilled, (state, action) => {
        state.defaults = action.payload
      })
  },
})

export default slice.reducer
