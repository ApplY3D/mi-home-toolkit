import { CommonModule } from '@angular/common'
import { Component, inject, model, output } from '@angular/core'
import { injectMutation } from '@tanstack/angular-query-experimental'
import { countries } from '../../constants'
import { AuthService } from '../../auth.service'
import { DialogDirective } from '../dialog.directive'

@Component({
  selector: 'app-set-country-dialog',
  template: ` <dialog class="modal" app-dialog [visible]="visible()">
    <form
      class="modal-box"
      (submit)="$event.preventDefault(); setCountry(select.value)"
    >
      <button
        type="button"
        (click)="visible.set(false)"
        [disabled]="countryMutation.isPending()"
        class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
      >
        ✕
      </button>

      <h3 class="font-bold text-lg mb-4">Country</h3>

      <div class="flex justify-around">
        <select
          #select
          [disabled]="countryMutation.isPending()"
          class="select select-bordered w-full max-w-xs"
        >
          <option disabled>Country</option>
          <option *ngFor="let country of countries" [value]="country[0]">
            {{ country[1] }}
          </option>
        </select>

        <button
          class="btn"
          type="submit"
          [disabled]="countryMutation.isPending()"
        >
          Submit
        </button>
      </div>
    </form>
  </dialog>`,
  styles: [``],
  imports: [CommonModule, DialogDirective],
})
export class SetCountryDialogComponent {
  visible = model(false)
  countryChanged = output()

  authService = inject(AuthService)

  countryMutation = injectMutation(() => ({
    mutationFn: (country: string) => this.authService.setCountry(country),
    onSuccess: () => (this.visible.set(false), this.countryChanged.emit()),
  }))

  setCountry(country: string) {
    if (!country || this.countryMutation.isPending()) return
    this.countryMutation.mutate(country)
  }

  countries = countries
}
