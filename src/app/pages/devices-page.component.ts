import { CommonModule } from '@angular/common'
import {
  Component,
  ElementRef,
  effect,
  inject,
  signal,
  untracked,
  viewChild,
} from '@angular/core'
import { DeviceComponent } from '../card/device.component'
import { MiService } from '../mi.service'
import { Device } from '../types'
import { IconComponent } from '../icon/icon.component'
import { AuthService } from '../auth.service'
import { toSignal } from '@angular/core/rxjs-interop'
import { map } from 'rxjs'
import { countries, countryCodeToName } from '../constants'

@Component({
  standalone: true,
  template: `
    <div class="tooltip fixed right-4 bottom-4 z-[1]" data-tip="Refresh">
      <button
        class="btn btn-circle btn-outline"
        (click)="refresh()"
        [disabled]="loading()"
      >
        <span *ngIf="loading()" class="loading loading-spinner loading-md">
        </span>
        <span *ngIf="!loading()"><app-icon icon="refresh"></app-icon> </span>
      </button>
    </div>

    <div class="p-4 {{ loading() && 'pointer-events-none opacity-60' }}">
      @for (device of devices; track device.did) {
        <app-device class="mb-2" [device]="device"></app-device>
      } @empty {
        <div class="text-center text-gray-500">
          @if (loading()) {
            Loading...
          } @else if (loaded()) {
            No devices found for {{ country() }}.
            <div>
              <a (click)="dialogVisible.set(true)" class="link">
                Change country
              </a>
            </div>
          }
        </div>
      }

      <dialog #changeCountryDialog class="modal" [open]="">
        <form
          class="modal-box"
          (submit)="$event.preventDefault(); setCountry(select.value)"
        >
          <button
            (click)="dialogVisible.set(false)"
            type="button"
            class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
          >
            ✕
          </button>

          <h3 class="font-bold text-lg mb-4">Country</h3>

          <div class="flex justify-around">
            <select #select class="select select-bordered w-full max-w-xs">
              <option disabled>Country</option>
              <option *ngFor="let country of countries" [value]="country[0]">
                {{ country[1] }}
              </option>
            </select>

            <button class="btn" type="submit">Button</button>
          </div>
        </form>
      </dialog>
    </div>
  `,
  styles: [``],
  imports: [CommonModule, DeviceComponent, IconComponent],
})
export class DevicesPageComponent {
  dialog = viewChild<ElementRef<HTMLDialogElement>>('changeCountryDialog')
  dialogVisible = signal(false)
  openDialogEffect = effect(() => {
    const isVisible = this.dialogVisible()
    const dialog = untracked(() => this.dialog()?.nativeElement)
    isVisible ? dialog?.showModal() : dialog?.close()
  })

  miService = inject(MiService)
  authService = inject(AuthService)

  countries = countries
  country = toSignal(
    this.authService.user$.pipe(
      map((u) => countryCodeToName.get(u?.country as 'ru'))
    )
  )

  loaded = signal(false)
  loading = signal(false)
  devices: Device[] = []

  async setCountry(country: string) {
    await this.authService.setCountry(country)
    this.refresh()
    this.dialogVisible.set(false)
  }

  refresh() {
    if (this.loading()) return
    this.loading.set(true)
    this.miService
      .getDevices()
      .then((d) => (this.devices = d))
      .then(() => this.loaded.set(true))
      .finally(() => this.loading.set(false))
  }

  ngOnInit() {
    this.refresh()
  }
}
