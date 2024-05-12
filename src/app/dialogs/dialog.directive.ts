import {
  DestroyRef,
  Directive,
  ElementRef,
  EventEmitter,
  NgZone,
  Output,
  effect,
  inject,
  input,
} from '@angular/core'
import {
  outputFromObservable,
  takeUntilDestroyed,
} from '@angular/core/rxjs-interop'
import { fromEvent, map, merge } from 'rxjs'

@Directive({
  standalone: true,
  selector: 'dialog[app-dialog]',
})
export class DialogDirective {
  visible = input(false)
  dr = inject(DestroyRef)
  zone = inject(NgZone)
  elementRef = inject<ElementRef<HTMLDialogElement>>(ElementRef)

  @Output() onBackdropClick = new EventEmitter<void>()
  @Output() onEscapeClick = new EventEmitter<void>()
  onClose = outputFromObservable(
    merge(
      this.onBackdropClick.pipe(map(() => 'backdrop' as const)),
      this.onEscapeClick.pipe(map(() => 'esc' as const))
    )
  )

  constructor() {
    effect(() => {
      const isVisible = this.visible()
      const dialog = this.elementRef.nativeElement
      isVisible ? dialog.showModal() : dialog.close()
    })

    this.zone.runOutsideAngular(() => {
      fromEvent(this.elementRef.nativeElement, 'click')
        .pipe(takeUntilDestroyed(this.dr))
        .subscribe((e) => {
          const dialogElement = e.currentTarget
          const isClickedOnBackDrop = e.target === dialogElement
          if (isClickedOnBackDrop) this.onBackdropClick.next()
        })

      fromEvent(this.elementRef.nativeElement, 'cancel')
        .pipe(takeUntilDestroyed(this.dr))
        .subscribe((e) => (e.preventDefault(), this.onEscapeClick.next()))
    })
  }
}
