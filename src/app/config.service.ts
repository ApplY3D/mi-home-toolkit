import { Injectable } from '@angular/core'
import { BehaviorSubject } from 'rxjs'
import { getCurrent } from '@tauri-apps/api/window'

@Injectable({
  providedIn: 'root',
})
export class ConfigService {
  systemTheme$ = new BehaviorSubject<'dark' | 'light'>('dark')

  async init() {
    getCurrent().onThemeChanged(
      ({ payload: theme }) => theme && this.systemTheme$.next(theme)
    )

    await getCurrent()
      .theme()
      .then((theme) => theme && this.systemTheme$.next(theme))
  }

  static factory(configService: ConfigService) {
    return () => configService.init()
  }
}
