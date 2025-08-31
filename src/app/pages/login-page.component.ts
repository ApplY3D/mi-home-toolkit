import { CommonModule, NgOptimizedImage } from '@angular/common'
import { Component, computed, effect, inject, signal } from '@angular/core'
import { FormBuilder, FormsModule, ReactiveFormsModule, Validators } from '@angular/forms'
import { Router } from '@angular/router'
import { injectMutation } from '@tanstack/angular-query-experimental'
import { AuthService } from '../auth.service'
import { DialogDirective } from '../dialogs/dialog.directive'
import { IconComponent } from '../icon/icon.component'
import { MiService } from '../mi.service'

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
          placeholder="Email/Phone/Xiaomi Account"
          autocorrect="off"
          autocapitalize="none"
        />
      </label>

      <label class="input flex items-center gap-2">
        <span class="label"><app-icon icon="password" class="w-4 h-4" /></span>
        <input [formControlName]="'password'" type="password" placeholder="Password" />
      </label>

      <select class="select" [formControlName]="'country'">
        <option disabled>Server location</option>
        <option *ngFor="let country of countries()" [value]="country[0]">
          {{ country[1] }}
        </option>
        <option disabled>If not listed, try matching from above</option>
      </select>

      <button [disabled]="form.invalid || loading()" type="submit" class="btn w-full">
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
    </dialog>

    <dialog class="modal" app-dialog [visible]="!!twoFactorUrl()">
      <form class="modal-box w-auto" (ngSubmit)="submitTwoFactor(twoFactorInput())">
        <button
          type="button"
          (click)="cancelTwoFactor()"
          class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
        >
          ✕
        </button>

        <h3 class="font-bold text-lg mb-4">Two factor authentication</h3>

        <p class="mb-4">To continue, please get a verification code from Xiaomi.</p>

        <div class="w-full mb-4">
          <label class="label">
            <span class="label-text font-bold">Step 1: Request your code</span>
          </label>
          <p class="text-sm opacity-70 mb-2">
            Click the link below and follow the instructions to receive your code.
          </p>
          @if (twoFactorUrl(); as twoFactorUrlValue) {
            <a
              target="_blank"
              [href]="twoFactorUrlValue"
              class="link link-primary w-full block overflow-hidden whitespace-nowrap text-ellipsis"
            >
              {{ twoFactorUrlValue }}
            </a>
          }
        </div>

        <div role="alert" class="alert alert-warning alert-soft my-4">
          <app-icon icon="danger" class="flex-shrink-0 size-6" />
          <span>
            <b>Important:</b> After receiving the code, <b>return to this page</b> to
            enter it below. Do not enter it on the Xiaomi website.
          </span>
        </div>

        <label class="label">
          <span class="label-text font-bold">Step 2: Enter the code here</span>
        </label>
        <div class="flex gap-2 justify-between">
          <div class="w-full">
            <label
              class="input flex items-center gap-2 w-full"
              [ngClass]="{ 'input-error': !!twoFactorError() }"
            >
              <input
                [(ngModel)]="twoFactorInput"
                (ngModelChange)="twoFactorError.set(null)"
                type="text"
                placeholder="2FA code"
                autocorrect="off"
                autocomplete="one-time-code"
                autocapitalize="none"
                maxlength="16"
                [ngModelOptions]="{ standalone: true }"
              />
            </label>
            @if (twoFactorError(); as twoFactorErrorValue) {
              <p class="validator-hint text-(--color-error)">
                {{ twoFactorErrorValue }}
              </p>
            }
          </div>

          <button type="submit" class="btn" [disabled]="!twoFactorInput()">Submit</button>
        </div>
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
    mutationFn: (credentials: { email: string; password: string; country?: string }) =>
      this.authService.login(
        credentials,
        this.captchaHandler.bind(this),
        this.twoFactorHandler.bind(this)
      ),
    onSuccess: () => this.router.navigateByUrl('devices'),
  }))
  loading = computed(() => this.loginMutation.isPending())

  countries = this.miService.countries.value

  captcha = signal<string | null>(null)
  captchaInput = signal('')
  transparentPx =
    'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII='

  twoFactorUrl = signal<string | null>(null)
  twoFactorInput = signal('')
  twoFactorError = signal<string | null>(null)

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

  twoFactorHandler([value, error]: [value: string, error?: string]) {
    if (!error) this.twoFactorInput.set('')
    this.twoFactorError.set(error || null)
    this.twoFactorUrl.set(value)
  }
  submitTwoFactor(value: string) {
    this.twoFactorUrl.set('')
    this.authService.solveTwoFactor(value)
  }
  cancelTwoFactor() {
    this.twoFactorUrl.set('')
    this.authService.cancelTwoFactor()
  }
}
