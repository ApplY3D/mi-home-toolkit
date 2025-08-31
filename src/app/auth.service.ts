import { Injectable, inject } from '@angular/core'
import { MiService } from './mi.service'
import { BehaviorSubject, map } from 'rxjs'
import { toSignal } from '@angular/core/rxjs-interop'
import { emit, listen } from '@tauri-apps/api/event'

type User = { email: string; country?: string }

@Injectable({
  providedIn: 'root',
})
export class AuthService {
  miService = inject(MiService)

  user$ = new BehaviorSubject<User | null>(null)
  user = toSignal(this.user$)
  loggedIn$ = this.user$.pipe(map(Boolean))

  async setCountry(country: string) {
    const res = this.miService.setCountry(country)
    this.user$.next({ ...this.user$.value!, country })
    return res
  }

  async login(
    creds: { email: string; password: string; country?: string },
    captchaHandler?: (value: string) => void
  ) {
    const unsub = await listen<string>('captcha_requested', (event) => {
      captchaHandler?.(event.payload)
    })
    const res = await this.miService.login(creds).finally(() => unsub())
    this.user$.next(creds)
    return res
  }

  solveCaptcha(value: string) {
    return emit('captcha_solved', value)
  }

  refreshCaptcha() {
    return emit('captcha_solved', '')
  }

  cancelCaptcha() {
    return emit('captcha_solved', 'CANCEL')
  }
}
