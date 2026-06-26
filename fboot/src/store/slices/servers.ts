import { createAsyncThunk, createSlice, type PayloadAction } from '@reduxjs/toolkit'
import { api } from '@/api'
import type {
  BootConfig,
  BootDev,
  IpmiCreds,
  NewServer,
  PowerAction,
  Server,
  ServerStatus,
  UpdateBootConfig,
  UpdateServer,
} from '@/api'

export interface ServersState {
  byId: Record<string, Server>
  ids: string[]
  statuses: Record<string, ServerStatus>
  bootConfigs: Record<string, BootConfig>
  ipmi: Record<string, IpmiCreds>
  loading: boolean
  error: string | null
}

const initialState: ServersState = {
  byId: {},
  ids: [],
  statuses: {},
  bootConfigs: {},
  ipmi: {},
  loading: false,
  error: null,
}

export const fetchServers = createAsyncThunk('servers/fetch', () => api.servers.list())

export const createServer = createAsyncThunk('servers/create', (body: NewServer) =>
  api.servers.create(body),
)

export const updateServer = createAsyncThunk(
  'servers/update',
  (args: { id: string; patch: UpdateServer }) => api.servers.update(args.id, args.patch),
)

export const removeServer = createAsyncThunk('servers/remove', async (id: string) => {
  await api.servers.remove(id)
  return id
})

export const setServerMeta = createAsyncThunk(
  'servers/setMeta',
  async (args: { id: string; key: string; value: string }) => {
    await api.servers.setMetadata(args.id, args.key, args.value)
    return api.servers.get(args.id)
  },
)

export const deleteServerMeta = createAsyncThunk(
  'servers/deleteMeta',
  async (args: { id: string; key: string }) => {
    await api.servers.deleteMetadata(args.id, args.key)
    return api.servers.get(args.id)
  },
)

export const fetchIpmi = createAsyncThunk('servers/fetchIpmi', async (id: string) => {
  try {
    const creds = await api.servers.getIpmi(id)
    return { id, creds }
  } catch {
    return { id, creds: {} as IpmiCreds }
  }
})

export const saveIpmi = createAsyncThunk(
  'servers/saveIpmi',
  async (args: { id: string; creds: IpmiCreds }) => {
    const creds = await api.servers.setIpmi(args.id, args.creds)
    return { id: args.id, creds }
  },
)

export const fetchBootConfig = createAsyncThunk('servers/fetchBoot', (id: string) =>
  api.boot.get(id),
)

export const updateBootConfig = createAsyncThunk(
  'servers/updateBoot',
  (args: { id: string; patch: UpdateBootConfig }) => api.boot.update(args.id, args.patch),
)

export const powerAction = createAsyncThunk(
  'servers/power',
  async (args: { id: string; action: PowerAction }) => {
    const res = await api.servers.power(args.id, args.action)
    return { id: args.id, power: res.power }
  },
)

export const setBootDev = createAsyncThunk(
  'servers/bootdev',
  async (args: { id: string; dev: BootDev }) => {
    await api.servers.setBootDev(args.id, args.dev)
    return args
  },
)

function upsert(state: ServersState, server: Server) {
  if (!state.byId[server.id]) state.ids.push(server.id)
  state.byId[server.id] = server
}

function remove(state: ServersState, id: string) {
  delete state.byId[id]
  delete state.statuses[id]
  delete state.bootConfigs[id]
  delete state.ipmi[id]
  state.ids = state.ids.filter((x) => x !== id)
}

const slice = createSlice({
  name: 'servers',
  initialState,
  reducers: {
    serverUpserted(state, action: PayloadAction<Server>) {
      upsert(state, action.payload)
    },
    serverRemoved(state, action: PayloadAction<string>) {
      remove(state, action.payload)
    },
    statusChanged(state, action: PayloadAction<ServerStatus>) {
      state.statuses[action.payload.server_id] = action.payload
    },
    bootConfigChanged(state, action: PayloadAction<BootConfig>) {
      state.bootConfigs[action.payload.server_id] = action.payload
    },
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchServers.pending, (state) => {
        state.loading = true
        state.error = null
      })
      .addCase(fetchServers.fulfilled, (state, action) => {
        state.loading = false
        state.byId = {}
        state.ids = []
        for (const server of action.payload) upsert(state, server)
      })
      .addCase(fetchServers.rejected, (state, action) => {
        state.loading = false
        state.error = action.error.message ?? 'Failed to load servers'
      })
      .addCase(createServer.fulfilled, (state, action) => {
        upsert(state, action.payload)
      })
      .addCase(updateServer.fulfilled, (state, action) => {
        upsert(state, action.payload)
      })
      .addCase(setServerMeta.fulfilled, (state, action) => {
        upsert(state, action.payload)
      })
      .addCase(deleteServerMeta.fulfilled, (state, action) => {
        upsert(state, action.payload)
      })
      .addCase(removeServer.fulfilled, (state, action) => {
        remove(state, action.payload)
      })
      .addCase(fetchIpmi.fulfilled, (state, action) => {
        state.ipmi[action.payload.id] = action.payload.creds
      })
      .addCase(saveIpmi.fulfilled, (state, action) => {
        state.ipmi[action.payload.id] = action.payload.creds
      })
      .addCase(fetchBootConfig.fulfilled, (state, action) => {
        state.bootConfigs[action.payload.server_id] = action.payload
      })
      .addCase(updateBootConfig.fulfilled, (state, action) => {
        state.bootConfigs[action.payload.server_id] = action.payload
      })
  },
})

export const { serverUpserted, serverRemoved, statusChanged, bootConfigChanged } = slice.actions
export default slice.reducer
