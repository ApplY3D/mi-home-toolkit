import { Component, computed, effect, inject, model } from '@angular/core'
import { injectMutation } from '@tanstack/angular-query-experimental'
import { MiService } from '../../mi.service'
import { CommonModule } from '@angular/common'
import { DialogDirective } from '../dialog.directive'
import { FormBuilder, FormsModule, ReactiveFormsModule } from '@angular/forms'

@Component({
  standalone: true,
  selector: 'app-execute-command-dialog',
  template: ` <dialog class="modal" app-dialog [visible]="visible()">
    <form
      class="modal-box"
      [formGroup]="form"
      (submit)="$event.preventDefault(); executeCommand()"
    >
      <button
        type="button"
        (click)="device.set(null)"
        [disabled]="callDeviceMutation.isPending()"
        class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2"
      >
        ✕
      </button>

      <h3 class="font-bold text-lg mb-4">
        {{ device()?.name }} - Execute command
      </h3>

      <div class="flex flex-col gap-2 items-stretch">
        <input
          type="text"
          placeholder="Method"
          spellcheck="false"
          [formControlName]="'method'"
          class="input input-bordered w-full"
        />

        <textarea
          class="textarea textarea-bordered"
          [formControlName]="'params'"
          spellcheck="false"
          autocorrect="off"
          placeholder="Params"
        ></textarea>

        <textarea
          [readonly]="true"
          class="textarea textarea-bordered"
          [ngClass]="{
            'textarea-error': callDeviceMutation.isError(),
            'textarea-success': callDeviceMutation.isSuccess()
          }"
          placeholder="Result"
          spellcheck="false"
          autocorrect="off"
          [ngModel]="form.controls.result.value"
          [ngModelOptions]="{ standalone: true }"
        ></textarea>
      </div>

      <button
        class="btn mt-4"
        type="submit"
        [disabled]="callDeviceMutation.isPending()"
      >
        Execute
      </button>
    </form>
  </dialog>`,
  styles: [``],
  imports: [CommonModule, DialogDirective, FormsModule, ReactiveFormsModule],
})
export class ExecuteCommandDialogComponent {
  fb = inject(FormBuilder)
  device = model<{ did: number | string; name: string } | null>(null)
  did = computed(() => this.device()?.did)
  visible = computed(() => !!this.device())

  miService = inject(MiService)

  form = this.fb.group({
    method: '',
    params: '',
    result: '' as any,
  })

  openCloseEffect = effect(() => this.visible() && this.form.reset())

  callDeviceMutation = injectMutation(() => ({
    mutationFn: (data: {
      did: string
      method: string
      params?: string | null
    }) => this.miService.callDevice(data),
  }))

  callDeviceResultEffect = effect(() => {
    const data = this.callDeviceMutation.data()
    const error = this.callDeviceMutation.error()
    const isError = this.callDeviceMutation.isError()
    const isPending = this.callDeviceMutation.isPending()

    const result = this.form.controls.result

    if (isPending) return result.setValue('Loading...')
    if (isError) return result.setValue(error || 'Error')
    return result.setValue(JSON.stringify(data))
  })

  executeCommand() {
    if (this.callDeviceMutation.isPending()) return
    const did = this.did()?.toString()
    const { method, params } = this.form.value
    if (!did || !method) return
    this.callDeviceMutation.mutate({ did, method, params })
  }
}
