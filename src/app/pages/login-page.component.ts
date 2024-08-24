import { CommonModule } from '@angular/common'
import { Component, computed, effect, inject, signal } from '@angular/core'
import { IconComponent } from '../icon/icon.component'
import {
  FormBuilder,
  FormsModule,
  ReactiveFormsModule,
  Validators,
} from '@angular/forms'
import { AuthService } from '../auth.service'
import { Router } from '@angular/router'
import { countries } from '../constants'
import { injectMutation } from '@tanstack/angular-query-experimental'

@Component({
  standalone: true,
  template: `<form
    class="flex flex-col gap-2 w-80"
    (ngSubmit)="login($event)"
    [formGroup]="form"
  >
    <label class="input input-bordered flex items-center gap-2">
      <app-icon icon="email" class="w-4 h-4 opacity-70"></app-icon>
      <input
        class="w-full"
        [formControlName]="'email'"
        type="text"
        placeholder="Login"
      />
    </label>

    <label class="input input-bordered flex items-center gap-2">
      <app-icon icon="password" class="w-4 h-4 opacity-70"></app-icon>
      <input
        class="w-full"
        [formControlName]="'password'"
        type="password"
        placeholder="Password"
      />
    </label>

    <select
      class="select select-bordered w-full max-w-xs"
      [formControlName]="'country'"
    >
      <option disabled>Country</option>
      <option *ngFor="let country of countries" [value]="country[0]">
        {{ country[1] }}
      </option>
    </select>

    <button
      [disabled]="form.invalid || loading()"
      type="submit"
      class="btn w-full"
    >
      <app-icon *ngIf="!loading()" icon="login" class="w-4 h-4" />
      <span *ngIf="loading()" class="loading loading-spinner loading-xs"></span>
      Login
    </button>

    @if (loginMutation.isError()) {
      <div class="toast toast-center">
        <div role="alert" class="alert alert-error">
          <div></div>
          <div>
            <h3 class="font-bold">Login failed</h3>
            <div class="text-xs">{{ loginMutation.error() }}</div>
          </div>
          <button
            type="button"
            class="btn btn-sm btn-circle btn-ghost"
            (click)="loginMutation.reset()"
          >
            ✕
          </button>
        </div>
      </div>
    }
  </form>`,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      min-height: 100%;
      justify-content: center;
      align-items: center;
    }
  `,
  imports: [CommonModule, IconComponent, FormsModule, ReactiveFormsModule],
})
export class LoginPageComponent {
  fb = inject(FormBuilder)
  router = inject(Router)
  authService = inject(AuthService)
  loginMutation = injectMutation(() => ({
    mutationFn: (credentials: {
      email: string
      password: string
      country?: string
    }) => this.authService.login(credentials),
    onSuccess: () => this.router.navigateByUrl('devices'),
  }))
  loading = computed(() => this.loginMutation.isPending())

  countries = countries

  form = this.fb.nonNullable.group({
    email: this.fb.nonNullable.control('', [Validators.required]),
    password: this.fb.nonNullable.control('', [Validators.required]),
    country: this.fb.nonNullable.control('cn', [Validators.required]),
  })

  disabledFormEffect = effect(() =>
    this.loading() ? this.form.disable() : this.form.enable()
  )

  async login(event: SubmitEvent) {
    event.preventDefault()
    if (this.form.invalid) return
    const { email, password, country } = this.form.value
    if (!email || !password) return
    this.loginMutation.mutate({ email, password, country })
  }
}
