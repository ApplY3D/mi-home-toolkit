import { ApplicationConfig, inject, provideAppInitializer } from '@angular/core'
import { provideRouter } from '@angular/router'
import { routes } from './app.routes'
import { ConfigService } from './config.service'
import {
  provideTanStackQuery,
  QueryClient,
  withDevtools,
} from '@tanstack/angular-query-experimental'

export const appConfig: ApplicationConfig = {
  providers: [
    provideRouter(routes),
    provideTanStackQuery(
      new QueryClient(),
      withDevtools(() => ({
        loadDevtools: 'auto',
        buttonPosition: 'bottom-left',
      }))
    ),
    provideAppInitializer(() => {
      const initializerFn = ConfigService.factory(inject(ConfigService))
      return initializerFn()
    }),
  ],
}
