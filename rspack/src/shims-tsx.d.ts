import { VNode } from 'vue'

declare global {
  namespace JSX {
    interface Element extends VNode {}
    interface ElementClass {
      $props: {}
    }
    interface IntrinsicElements {
      [elem: string]: any
    }
  }
} 