import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  OnInit,
  inject,
  input,
} from '@angular/core'
import { Icon } from './icon.types'

@Component({
  standalone: true,
  selector: 'app-icon',
  template: ``,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class IconComponent implements OnInit {
  el = inject(ElementRef)

  icon = input.required<Icon>()

  ngOnInit() {
    fetch(`/assets/icons/${this.icon()}.svg`)
      .then((r) => r.text())
      .then((i) => (this.el.nativeElement.innerHTML = i))
  }
}
