import { CommonModule, NgOptimizedImage } from '@angular/common'
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
import { injectMutation } from '@tanstack/angular-query-experimental'
import { MiService } from '../mi.service'
import { DialogDirective } from '../dialogs/dialog.directive'

@Component({
  template: `<form
      class="flex flex-col gap-2 w-80"
      (ngSubmit)="login($event)"
      [formGroup]="form"
    >
      <label class="input flex items-center gap-2">
        <span class="label"><app-icon icon="email" class="w-4 h-4" /></span>
        <input
          [formControlName]="'email'"
          type="text"
          placeholder="Login"
          autocorrect="off"
          autocapitalize="none"
        />
      </label>

      <label class="input flex items-center gap-2">
        <span class="label"><app-icon icon="password" class="w-4 h-4" /></span>
        <input
          [formControlName]="'password'"
          type="password"
          placeholder="Password"
        />
      </label>

      <select class="select" [formControlName]="'country'">
        <option disabled>Server location</option>
        <option *ngFor="let country of countries()" [value]="country[0]">
          {{ country[1] }}
        </option>
        <option disabled>If not listed, try matching from above</option>
      </select>

      <button
        [disabled]="form.invalid || loading()"
        type="submit"
        class="btn w-full"
      >
        <app-icon *ngIf="!loading()" icon="login" class="w-4 h-4" />
        <span
          *ngIf="loading()"
          class="loading loading-spinner loading-xs"
        ></span>
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
    </form>

    <dialog class="modal" app-dialog [visible]="!!captcha()">
      <form
        class="modal-box w-auto flex flex-col gap-2"
        (ngSubmit)="submitCaptcha(captchaInput())"
      >
        <button
          type="button"
          (click)="cancelCaptcha()"
          class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
        >
          ✕
        </button>
        @if (captcha(); as captchaImage) {
          <img class="mx-auto" width="125" height="45" [ngSrc]="captchaImage" />
        } @else {
          <img class="mx-auto" width="125" height="45" [src]="transparentPx" />
        }
        <label class="input flex items-center gap-2">
          <input
            [(ngModel)]="captchaInput"
            type="text"
            placeholder="Captcha"
            autocorrect="off"
            autocomplete="off"
            autocapitalize="none"
            [ngModelOptions]="{ standalone: true }"
          />
        </label>

        <button type="submit" class="btn w-full">
          {{ captchaInput() ? 'Submit' : 'Request new code' }}
        </button>
      </form>
    </dialog> `,
  styles: `
    :host {
      display: flex;
      flex-direction: column;
      min-height: 100%;
      justify-content: center;
      align-items: center;

      img {
        height: 45px;
      }
    }
  `,
  imports: [
    CommonModule,
    IconComponent,
    FormsModule,
    ReactiveFormsModule,
    NgOptimizedImage,
    DialogDirective,
  ],
})
export class LoginPageComponent {
  fb = inject(FormBuilder)
  router = inject(Router)
  authService = inject(AuthService)
  miService = inject(MiService)
  loginMutation = injectMutation(() => ({
    mutationFn: (credentials: {
      email: string
      password: string
      country?: string
    }) => this.authService.login(credentials, this.captchaHandler.bind(this)),
    onSuccess: () => this.router.navigateByUrl('devices'),
  }))
  loading = computed(() => this.loginMutation.isPending())

  countries = this.miService.countries.value

  captcha = signal<string | null>(null)
  captchaInput = signal('')
  transparentPx =
    'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII='

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

  captchaHandler(value: string) {
    this.captcha.set(value)
  }

  private resetCaptchaState() {
    this.captcha.set(null)
    this.captchaInput.set('')
  }

  submitCaptcha(value: string) {
    this.resetCaptchaState()
    this.authService.solveCaptcha(value)
  }

  refreshCaptcha() {
    this.resetCaptchaState()
    this.authService.refreshCaptcha()
  }

  cancelCaptcha() {
    this.resetCaptchaState()
    this.authService.cancelCaptcha()
  }
}
