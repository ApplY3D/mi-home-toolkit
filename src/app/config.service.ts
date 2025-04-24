import { Injectable } from '@angular/core'
import { BehaviorSubject } from 'rxjs'
import { getCurrentWindow } from '@tauri-apps/api/window'

@Injectable({
  providedIn: 'root',
})
export class ConfigService {
  systemTheme$ = new BehaviorSubject<'dark' | 'light'>('dark')

  async init() {
    const window = getCurrentWindow()

    window.onThemeChanged(
      ({ payload: theme }) => theme && this.systemTheme$.next(theme)
    )
    await window.theme().then((theme) => theme && this.systemTheme$.next(theme))
  }

  static factory(configService: ConfigService) {
    return () => configService.init()
  }
}
