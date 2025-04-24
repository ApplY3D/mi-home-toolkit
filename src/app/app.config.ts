import { ApplicationConfig, inject, provideAppInitializer } from '@angular/core'
import { provideRouter } from '@angular/router'
import { routes } from './app.routes'
import { ConfigService } from './config.service'
import {
  provideAngularQuery,
  QueryClient,
} from '@tanstack/angular-query-experimental'

export const appConfig: ApplicationConfig = {
  providers: [
    provideRouter(routes),
    provideAngularQuery(new QueryClient()),
    provideAppInitializer(() => {
      const initializerFn = ConfigService.factory(inject(ConfigService))
      return initializerFn()
    }),
  ],
}
