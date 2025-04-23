import { Injectable } from '@angular/core'
import { BehaviorSubject } from 'rxjs'
import { getCurrentWindow } from '@tauri-apps/api/window'

@Injectable({
  providedIn: 'root',
})
export class ConfigService {
  systemTheme$ = new BehaviorSubject<'dark' | 'light'>('dark')

  async init() {
    getCurrentWindow().onThemeChanged(
      ({ payload: theme }) => theme && this.systemTheme$.next(theme)
    )

    await getCurrentWindow()
      .theme()
      .then((theme) => theme && this.systemTheme$.next(theme))
  }

  static factory(configService: ConfigService) {
    return () => configService.init()
  }
}
