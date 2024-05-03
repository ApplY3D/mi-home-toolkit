import { Injectable, inject } from '@angular/core'
import { MiService } from './mi.service'
import { BehaviorSubject, map } from 'rxjs'

type User = { email: string; country?: string }

@Injectable({
  providedIn: 'root',
})
export class AuthService {
  miService = inject(MiService)

  user$ = new BehaviorSubject<User | null>(null)
  loggedIn$ = this.user$.pipe(map(Boolean))

  async setCountry(country: string) {
    const res = this.miService.setCountry(country)
    this.user$.next({ ...this.user$.value!, country })
    return res
  }

  async login(creds: { email: string; password: string; country?: string }) {
    const res = await this.miService.login(creds)
    this.user$.next(creds)
    return res
  }
}
