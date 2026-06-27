import type {
  ArpEntry,
  Bootable,
  BootableKind,
  BootableRole,
  BootConfig,
  BootDefaults,
  BootDev,
  CacheEntry,
  CacheNamespace,
  ConsoleStatus,
  ImportResult,
  IpmiCreds,
  NewBootable,
  NewServer,
  PowerAction,
  PowerStatus,
  Server,
  ServerExportOptions,
  ServerImportPayload,
  ServerRecord,
  StatsSample,
  UpdateBootConfig,
  UpdateServer,
} from './types'

export type GetToken = () => string | null | undefined | Promise<string | null | undefined>

export interface ApiClientOptions {
  getToken?: GetToken
  fetchImpl?: typeof fetch
}

export interface ApiError extends Error {
  status: number
}

export function createApiError(status: number, message: string): ApiError {
  const err = new Error(message) as ApiError
  err.name = 'ApiError'
  err.status = status
  return err
}

export function isApiError(value: unknown): value is ApiError {
  return value instanceof Error && value.name === 'ApiError'
}

function createRequester(baseUrl: string, opts: ApiClientOptions) {
  const root = baseUrl.replace(/\/$/, '')
  const doFetch = opts.fetchImpl ?? fetch

  async function authHeaders(): Promise<Record<string, string>> {
    if (!opts.getToken) return {}
    const token = await opts.getToken()
    return token ? { Authorization: `Bearer ${token}` } : {}
  }

  async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
    const headers: Record<string, string> = {
      ...(init.body && !(init.body instanceof FormData) ? { 'Content-Type': 'application/json' } : {}),
      ...(await authHeaders()),
      ...((init.headers as Record<string, string>) ?? {}),
    }
    const res = await doFetch(`${root}${path}`, { ...init, headers })
    if (!res.ok) {
      let message = res.statusText
      try {
        const body = await res.json()
        if (body && typeof body.error === 'string') message = body.error
      } catch {
        /* non-json error body */
      }
      throw createApiError(res.status, message)
    }
    if (res.status === 204) return undefined as T
    const text = await res.text()
    return (text ? JSON.parse(text) : undefined) as T
  }

  return { request, root }
}

export function createApiClient(baseUrl: string, getToken?: GetToken, options: ApiClientOptions = {}) {
  const { request, root } = createRequester(baseUrl, { getToken, ...options })

  const json = (body: unknown) => JSON.stringify(body)

  const servers = {
    list: () => request<Server[]>('/servers'),
    get: (id: string) => request<Server>(`/servers/${id}`),
    create: (body: NewServer) => request<Server>('/servers', { method: 'POST', body: json(body) }),
    update: (id: string, body: UpdateServer) =>
      request<Server>(`/servers/${id}`, { method: 'PATCH', body: json(body) }),
    remove: (id: string) => request<void>(`/servers/${id}`, { method: 'DELETE' }),
    setMetadata: (id: string, key: string, value: string) =>
      request<void>(`/servers/${id}/metadata/${encodeURIComponent(key)}`, {
        method: 'PUT',
        body: json({ value }),
      }),
    deleteMetadata: (id: string, key: string) =>
      request<void>(`/servers/${id}/metadata/${encodeURIComponent(key)}`, { method: 'DELETE' }),
    getIpmi: (id: string) => request<IpmiCreds>(`/servers/${id}/ipmi`),
    setIpmi: (id: string, body: IpmiCreds) =>
      request<IpmiCreds>(`/servers/${id}/ipmi`, { method: 'PUT', body: json(body) }),
    power: (id: string, action: PowerAction) =>
      request<{ power: PowerStatus }>(`/servers/${id}/power`, { method: 'POST', body: json({ action }) }),
    setBootDev: (id: string, dev: BootDev) =>
      request<void>(`/servers/${id}/bootdev`, { method: 'POST', body: json({ dev }) }),
  }

  const boot = {
    get: (id: string) => request<BootConfig>(`/servers/${id}/boot`),
    update: (id: string, body: UpdateBootConfig) =>
      request<BootConfig>(`/servers/${id}/boot`, { method: 'PATCH', body: json(body) }),
    ipxe: (id: string) => request<{ script: string }>(`/servers/${id}/ipxe`),
  }

  const bootDefaults = {
    get: () => request<BootDefaults>('/boot-defaults'),
    update: (body: BootDefaults) =>
      request<BootDefaults>('/boot-defaults', { method: 'PUT', body: json(body) }),
  }

  const bootables = {
    list: (kind?: BootableKind) =>
      request<Bootable[]>(`/bootables${kind ? `?kind=${kind}` : ''}`),
    get: (id: string) => request<Bootable>(`/bootables/${id}`),
    create: (body: NewBootable) => request<Bootable>('/bootables', { method: 'POST', body: json(body) }),
    update: (id: string, body: Partial<NewBootable>) =>
      request<Bootable>(`/bootables/${id}`, { method: 'PATCH', body: json(body) }),
    remove: (id: string) => request<void>(`/bootables/${id}`, { method: 'DELETE' }),
    upload: (id: string, role: BootableRole, file: File) => {
      const form = new FormData()
      form.append('file', file)
      return request<Bootable>(`/bootables/${id}/upload?role=${role}`, { method: 'POST', body: form })
    },
    setMetadata: (id: string, key: string, value: string) =>
      request<void>(`/bootables/${id}/metadata/${encodeURIComponent(key)}`, {
        method: 'PUT',
        body: json({ value }),
      }),
    deleteMetadata: (id: string, key: string) =>
      request<void>(`/bootables/${id}/metadata/${encodeURIComponent(key)}`, { method: 'DELETE' }),
  }

  const stats = {
    latest: () => request<StatsSample[]>('/stats'),
    history: (id: string, limit?: number) =>
      request<StatsSample[]>(`/stats/${id}${limit ? `?limit=${limit}` : ''}`),
  }

  const arp = {
    list: () => request<ArpEntry[]>('/arp'),
  }

  const cache = {
    view: () => request<CacheNamespace[]>('/cache'),
    entries: (ns: string) => request<CacheEntry[]>(`/cache/${encodeURIComponent(ns)}`),
    clear: (ns?: string) =>
      request<{ cleared: number }>(ns ? `/cache/${encodeURIComponent(ns)}` : '/cache', {
        method: 'DELETE',
      }),
  }

  const migration = {
    // tar.gz download — consumed via an anchor href, not fetch+JSON.
    exportUrl: () => `${root}/migration/export`,
    import: (file: File) => {
      const form = new FormData()
      form.append('file', file)
      return request<{ restarting: boolean }>('/migration/import', { method: 'POST', body: form })
    },
  }

  const serversIo = {
    export: (opts: ServerExportOptions) =>
      request<ServerRecord[]>('/servers/export', { method: 'POST', body: json(opts) }),
    import: (payload: ServerImportPayload) =>
      request<ImportResult>('/servers/import', { method: 'POST', body: json(payload) }),
  }

  const console = {
    status: (id: string) => request<ConsoleStatus>(`/servers/${id}/console`),
    kill: (id: string) => request<ConsoleStatus>(`/servers/${id}/console`, { method: 'DELETE' }),
  }

  return {
    servers,
    boot,
    bootDefaults,
    bootables,
    stats,
    arp,
    cache,
    migration,
    serversIo,
    console,
    baseUrl: root,
    request,
  }
}

export type ApiClient = ReturnType<typeof createApiClient>
