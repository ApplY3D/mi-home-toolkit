export type Device = {
  adminFlag: number
  bssid: string
  desc: string
  did: string
  extra: Record<string, any>
  family_id: number
  isOnline: Boolean
  latitude: string
  localip: string
  longitude: string
  mac: string
  model: string
  name: string
  p2p_id: string
  parent_id: string
  parent_model: string
  password: string
  pd_id: number
  permitLevel: number
  pid: string
  reset_flag: number
  rssi: number
  shareFlag: number
  show_mode: number
  ssid: string
  token: string
  uid: number
}

export type GetDevicesResponse = Device[]
