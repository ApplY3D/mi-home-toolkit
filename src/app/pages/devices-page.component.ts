import { CommonModule } from '@angular/common'
import {
  Component,
  computed,
  inject,
  signal,
  viewChildren,
} from '@angular/core'
import { DeviceComponent } from '../card/device.component'
import { MiService } from '../mi.service'
import { IconComponent } from '../icon/icon.component'
import { AuthService } from '../auth.service'
import { SetCountryDialogComponent } from '../dialogs/set-country-dialog/set-country-dialog.component'
import { injectQuery } from '@tanstack/angular-query-experimental'
import { ExecuteCommandDialogComponent } from '../dialogs/execute-command-dialog/execute-command-dialog.component'
import { Device } from '../types'

@Component({
  template: `
    <div class="tooltip fixed right-4 bottom-4 z-1" data-tip="Refresh">
      <button
        class="btn btn-circle btn-outline"
        (click)="devicesQuery.refetch()"
        [disabled]="devicesQuery.isFetching()"
      >
        @if (devicesQuery.isFetching()) {
          <span class="loading loading-spinner loading-md"> </span>
        } @else {
          <app-icon class="w-6 h-6" icon="refresh" />
        }
      </button>
    </div>

    <div
      class="p-4 {{
        devicesQuery.isFetching() && 'pointer-events-none opacity-60'
      }}"
    >
      @for (device of devicesQuery.data(); track device.did) {
        <app-device
          class="mb-2"
          [device]="device"
          (executeCommand)="executeCommandForDevice.set(device)"
        ></app-device>
      } @empty {
        <div class="text-center text-gray-500">
          @if (devicesQuery.isFetching()) {
            Loading...
          } @else if (devicesQuery.isFetched()) {
            No devices found for {{ country() }}.
            <div>
              <button
                (click)="changeCountryDialogVisible.set(true)"
                class="btn btn-link"
              >
                Change Server Location
              </button>
            </div>
          }
        </div>
      }
    </div>

    <app-set-country-dialog
      [(visible)]="changeCountryDialogVisible"
      (countryChanged)="devicesQuery.refetch()"
    />

    <app-execute-command-dialog
      [(device)]="executeCommandForDevice"
      (success)="invalidateDevice()"
    />
  `,
  styles: [``],
  imports: [
    CommonModule,
    DeviceComponent,
    IconComponent,
    SetCountryDialogComponent,
    ExecuteCommandDialogComponent,
  ],
})
export class DevicesPageComponent {
  executeCommandForDevice = signal<Device | null>(null)
  changeCountryDialogVisible = signal(false)

  miService = inject(MiService)
  authService = inject(AuthService)

  deviceComponents = viewChildren(DeviceComponent)

  devicesQuery = injectQuery(() => ({
    queryKey: ['devices'],
    queryFn: () => this.miService.getDevices(),
    staleTime: 1000 * 60 * 10,
    structuralSharing: false,
  }))

  country = computed(() => {
    const user = this.authService.user()
    if (!user?.country) return null
    return this.miService.countryCodeToName().get(user.country)
  })

  invalidateDevice() {
    const did = this.executeCommandForDevice()?.did
    if (!did) return
    const index = this.devicesQuery.data()?.findIndex((d) => d.did === did)
    if (typeof index === 'number' && index >= 0) {
      this.deviceComponents().at(index)?.refreshDevice()
    }
  }
}
