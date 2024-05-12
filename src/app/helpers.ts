import { ElementRef, Signal, effect, signal, untracked } from '@angular/core'

export function useDialog(
  dialogComponent: Signal<ElementRef<HTMLDialogElement> | undefined>
) {
  const dialogVisible = signal(false)

  effect(() => {
    const isVisible = dialogVisible()
    const dialog = untracked(() => dialogComponent()?.nativeElement)
    isVisible ? dialog?.showModal() : dialog?.close()
  })

  return {
    visible: dialogVisible.asReadonly(),
    setVisible: (visible: boolean) => dialogVisible.set(visible),
  }
}
