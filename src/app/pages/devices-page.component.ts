import { CommonModule } from '@angular/common'
import { Component, inject, signal } from '@angular/core'
import { DeviceComponent } from '../card/device.component'
import { MiService } from '../mi.service'
import { IconComponent } from '../icon/icon.component'
import { AuthService } from '../auth.service'
import { toSignal } from '@angular/core/rxjs-interop'
import { map } from 'rxjs'
import { countryCodeToName } from '../constants'
import { SetCountryDialogComponent } from '../dialogs/set-country-dialog/set-country-dialog.component'
import { injectQuery } from '@tanstack/angular-query-experimental'
import { ExecuteCommandDialogComponent } from '../dialogs/execute-command-dialog/execute-command-dialog.component'
import { Device } from '../types'

@Component({
  template: `
    <div class="tooltip fixed right-4 bottom-4 z-[1]" data-tip="Refresh">
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
              <a (click)="changeCountryDialogVisible.set(true)" class="link">
                Change country
              </a>
            </div>
          }
        </div>
      }
    </div>

    <app-set-country-dialog
      [(visible)]="changeCountryDialogVisible"
      (countryChanged)="devicesQuery.refetch()"
    />

    <app-execute-command-dialog [(device)]="executeCommandForDevice" />
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

  devicesQuery = injectQuery(() => ({
    queryKey: ['devices'],
    queryFn: () => this.miService.getDevices(),
    staleTime: 1000 * 60 * 10,
  }))

  country = toSignal(
    this.authService.user$.pipe(
      map((u) => countryCodeToName.get(u?.country as 'ru'))
    )
  )
}
