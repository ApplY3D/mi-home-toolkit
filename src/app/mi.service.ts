import { Injectable } from '@angular/core'
import { invoke } from '@tauri-apps/api/core'
import { GetDevicesResponse } from './types'

@Injectable({
  providedIn: 'root',
})
export class MiService {
  login(creds: { email: string; password: string }) {
    return invoke('login', creds)
  }

  setCountry(country: string) {
    return invoke('set_country', { country })
  }

  getDevices() {
    return invoke<GetDevicesResponse>('get_devices')
  }

  callDevice(data: { did: string; method: string; params?: string }) {
    const { did, method, params } = data
    return invoke('call_device', { did, method, params })
  }

  getProp({ did, name }: { did: string; name: string | string[] }) {
    const params = JSON.stringify(Array.isArray(name) ? name : [name])
    return this.callDevice({ did, method: 'get_prop', params })
  }

  setProp({ did, name, value }: { did: string; name: string; value: any }) {
    const params = JSON.stringify([name, value])
    return this.callDevice({ did, method: 'set_ps', params })
  }
}
