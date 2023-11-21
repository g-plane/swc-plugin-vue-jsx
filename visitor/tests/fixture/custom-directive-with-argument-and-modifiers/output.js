import {
  Fragment as _Fragment,
  createVNode as _createVNode,
  resolveComponent as _resolveComponent,
  resolveDirective as _resolveDirective,
  withDirectives as _withDirectives,
} from "vue";
_createVNode(_Fragment, null, [
  _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x]]),
  _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x]]),
  _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, 'y']]),
  _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, 'y', {
    a: true,
    b: true
  }]]),
  _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, void 0, {
    a: true,
    b: true
  }]]),
  _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, y, {
    a: true,
    b: true
  }]]),
  _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, y, {
    a: true,
    b: true
  }]]),
]);
