import { createApiClient } from './client'
import { createWsClient } from './ws'

export * from './types'
export * from './client'
export * from './ws'

const getToken = () => localStorage.getItem('fboot.token')

const apiBase = import.meta.env.VITE_API_BASE || '/api'
const wsBase = import.meta.env.VITE_WS_BASE || ''

export const api = createApiClient(apiBase, getToken)
export const ws = createWsClient(getToken, wsBase)
