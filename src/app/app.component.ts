import { Component, inject } from '@angular/core'
import { RouterModule } from '@angular/router'
import { ConfigService } from './config.service'

@Component({
  selector: 'app-root',
  styles: `
    :host {
      display: block;
    }
  `,
  imports: [RouterModule],
  templateUrl: './app.component.html',
})
export class AppComponent {
  configService = inject(ConfigService)

  constructor() {
    this.configService.systemTheme$.subscribe((systemTheme) => {
      document
        .getElementsByTagName('html')[0]
        .setAttribute('data-theme', systemTheme)
    })
  }
}
