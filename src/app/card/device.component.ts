import {
  Component,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
} from '@angular/core'
import { Device } from '../types'
import { CommonModule } from '@angular/common'
import { deviceToImageMap } from '../constants'
import { IconComponent } from '../icon/icon.component'
import { MiService } from '../mi.service'
import { FormsModule } from '@angular/forms'
import { injectQuery } from '@tanstack/angular-query-experimental'

@Component({
  selector: 'app-device',
  template: `
    @if (deviceComputed(); as device) {
      <div class="card min-[550px]:card-side bg-base-100 shadow-xl">
        <figure class="shrink-0" [ngClass]="{ 'opacity-40': !device.isOnline }">
          <img
            class="!object-contain"
            style="width: 168px;"
            src="{{ deviceImage() }}"
          />
          <div class="absolute left-4 top-4 w-6">
            <app-icon *ngIf="device.isOnline" icon="wifi" class="w-5 h-5" />
            <app-icon
              *ngIf="!device.isOnline"
              icon="wifi_off"
              class="w-6 h-6"
            />
          </div>
        </figure>
        <div class="card-body">
          <h2 class="card-title">
            {{ device.name }}

            <div class="tooltip self-start" data-tip="Refresh">
              <button
                (click)="refreshDevice()"
                class="btn btn-sm btn-circle btn-outline"
              >
                <app-icon class="w-5 h-5" icon="refresh" />
              </button>
            </div>

            <div class="tooltip self-start" data-tip="Execute command">
              <button
                *ngIf="device.isOnline"
                (click)="executeCommand.emit()"
                class="btn btn-sm btn-circle btn-outline"
              >
                <app-icon class="w-5 h-5" icon="terminal" />
              </button>
            </div>
          </h2>
          <p>ID: {{ device.did }}</p>
          <p>IP: {{ device.localip }}</p>
          <p>MAC: {{ device.mac }}</p>
          <p>Model: {{ device.model }}</p>
          <p>Token: {{ device.token }}</p>

          <label
            class="label cursor-pointer justify-start text-inherit"
            *ngIf="lanModeAvailable()"
            (click)="lanModeChange()"
          >
            LAN Mode

            @if (lanModeLoading()) {
              <span class="loading loading-spinner loading-md"></span>
            } @else {
              <input
                [ngModel]="lanMode()"
                type="checkbox"
                checked="checked"
                class="toggle"
              />
            }
          </label>
        </div>
      </div>
    }
  `,
  styles: `
    :host {
      display: block;
    }
  `,
  imports: [CommonModule, IconComponent, FormsModule],
})
export class DeviceComponent {
  miService = inject(MiService)

  executeCommand = output()

  device = input.required<Device>()
  deviceQuery = injectQuery(() => {
    const device = this.device()
    return {
      queryFn: () => this.miService.getDevice(device.did).then((d) => d!),
      queryKey: ['device', device.did],
      initialData: device,
      enabled: false,
    }
  })
  private source = signal<'input' | 'query'>('input')
  private inputSourceEffect = effect(
    () => (this.device(), this.source.set('input')),
    { allowSignalWrites: true }
  )
  deviceComputed = computed(() =>
    this.source() === 'input' ? this.device() : this.deviceQuery.data()
  )
  refreshDevice() {
    this.deviceQuery.refetch().then(() => this.source.set('query'))
  }

  deviceImage = computed(() =>
    deviceToImageMap.get(this.deviceComputed().model)
  )

  lanModeUpdate = signal(0)
  lanMode = signal(false)
  lanModeAvailable = computed(
    () =>
      this.deviceComputed().isOnline &&
      this.deviceComputed().model.startsWith('yeelink.light')
  )
  lanModeLoading = signal(false)
  lanModeEffect = effect(
    () => {
      const device = untracked(() => this.deviceComputed())
      const lanModeLoading = untracked(() => this.lanModeLoading())
      const lanModeAvailable = this.lanModeAvailable()
      if (lanModeAvailable && !lanModeLoading) {
        this.lanModeLoading.set(true)
        this.miService
          .getProp({ did: device.did, name: ['lan_ctrl'] })
          .then((res) => this.lanMode.set((res as any)[0] === '1'))
          .finally(() => this.lanModeLoading.set(false))
      }
    },
    { allowSignalWrites: true }
  )
  lanModeChange() {
    const lanModeLoading = this.lanModeLoading()
    if (lanModeLoading) return
    this.lanModeLoading.set(true)
    const device = this.deviceComputed()
    const lanMode = this.lanMode()
    this.miService
      .setProp({
        did: device.did,
        name: 'cfg_lan_ctrl',
        value: lanMode ? '0' : '1',
      })
      .then(() => this.lanMode.set(!lanMode))
      .finally(() => this.lanModeLoading.set(false))
  }
}
