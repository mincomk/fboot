export type Uuid = string

export type PowerStatus = 'on' | 'off' | 'unknown'
export type BootDev = 'none' | 'pxe' | 'disk' | 'cdrom' | 'bios'
export type PowerAction = 'on' | 'off' | 'cycle' | 'status'
export type BootableKind = 'pxe' | 'linux'
export type BootableRole = 'image' | 'kernel' | 'initrd'
export type BootableSource = 'file' | 'url'

export interface Server {
  id: Uuid
  primary_mac?: string | null
  ipmi_mac?: string | null
  friendly_name: string
  hostname?: string | null
  metadata: Record<string, string>
  created_at: string
  updated_at: string
}

export interface NewServer {
  primary_mac?: string | null
  ipmi_mac?: string | null
  friendly_name: string
  hostname?: string | null
  metadata?: Record<string, string>
}

export interface UpdateServer {
  friendly_name?: string
  hostname?: string | null
  primary_mac?: string | null
  ipmi_mac?: string | null
}

export interface BootableFile {
  role: BootableRole
  source: BootableSource
  key?: string | null
  url?: string | null
  size?: number | null
}

export interface Bootable {
  id: Uuid
  kind: BootableKind
  name: string
  description?: string | null
  cmdline?: string | null
  files: BootableFile[]
  metadata: Record<string, string>
  created_at: string
}

export interface NewBootable {
  kind: BootableKind
  name: string
  description?: string | null
  cmdline?: string | null
  metadata?: Record<string, string>
}

export interface BootConfig {
  server_id: Uuid
  boot_pxe: boolean
  pxe_bootable_id?: Uuid | null
  linux_bootable_id?: Uuid | null
  cmdline_override?: string | null
  cmdline_append?: string | null
  ipxe_script?: string | null
}

export interface UpdateBootConfig {
  boot_pxe?: boolean
  pxe_bootable_id?: Uuid | null
  linux_bootable_id?: Uuid | null
  cmdline_override?: string | null
  cmdline_append?: string | null
  ipxe_script?: string | null
}

export interface BootDefaults {
  pxe_bootable_id?: Uuid | null
  linux_bootable_id?: Uuid | null
}

export interface IpmiCreds {
  host?: string | null
  username?: string | null
  password?: string | null
  cipher?: number | null
}

export interface ConsoleStatus {
  running: boolean
  clients: number
}

export interface StatsSample {
  server_id: Uuid
  ts: string
  power_status: PowerStatus
  power_w?: number | null
  cpu_temp_c?: number | null
}

export interface ArpEntry {
  ip: string
  mac: string
  hostname?: string | null
}

export interface ScanOptions {
  cidr?: string
  probe_ipmi?: boolean
  probe_ssh?: boolean
  ports?: number[]
}

export interface ScanResult {
  ip: string
  mac: string | null
  hostname: string | null
  board_info: string | null
  open_ports: number[]
  ipmi: boolean
  ssh: boolean
}

export type ScanEvent =
  | ({ type: 'result' } & ScanResult)
  | { type: 'progress'; scanned: number; total: number }
  | { type: 'done' }

export type ServerEvent =
  | { type: 'server_added'; server: Server }
  | { type: 'server_updated'; server: Server }
  | { type: 'server_removed'; id: Uuid }
  | { type: 'status_changed'; status: ServerStatus }
  | { type: 'stats_updated'; sample: StatsSample }
  | { type: 'boot_config_changed'; config: BootConfig }
  | { type: 'console_status_changed'; server_id: Uuid; status: ConsoleStatus }

export interface ServerStatus {
  server_id: Uuid
  online: boolean
  ip: string | null
  ipmi_ip: string | null
  ipmi_reachable: boolean
}
