import { inject } from '@angular/core'
import { CanActivateFn, Router, Routes } from '@angular/router'
import { AuthService } from './auth.service'
import { tap } from 'rxjs'

const loggedInGuard = () => {
  const authService = inject(AuthService)
  return authService.loggedIn$
}

const loggedInGuardWithRedirect =
  (url: string): CanActivateFn =>
  () => {
    const router = inject(Router)
    return loggedInGuard().pipe(
      tap((loggedIn) => !loggedIn && router.navigateByUrl(url))
    )
  }

const notLoggedInGuardWithRedirect =
  (url: string): CanActivateFn =>
  () => {
    const router = inject(Router)
    return loggedInGuard().pipe(
      tap((loggedIn) => loggedIn && router.navigateByUrl(url))
    )
  }

export const routes: Routes = [
  {
    path: 'login',
    loadComponent: () =>
      import('./pages/login-page.component').then((m) => m.LoginPageComponent),
  },
  {
    path: 'devices',
    loadComponent: () =>
      import('./pages/devices-page.component').then(
        (m) => m.DevicesPageComponent
      ),
    canActivate: [loggedInGuardWithRedirect('login')],
  },
  {
    path: '**',
    redirectTo: 'devices',
    pathMatch: 'full',
  },
]
