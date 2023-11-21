import {
  createTextVNode as _createTextVNode,
  createVNode as _createVNode,
  resolveComponent as _resolveComponent,
} from "vue";
_createVNode("div", null, [
  _createVNode(_resolveComponent("Comp"), null, {
    default: () => [_createTextVNode("content")],
    ...slots,
    _: 1,
  }),
  _createVNode(_resolveComponent("Comp"), null, {
    default: () => [_createTextVNode("content")],
    a: b,
    _: 1,
  }),
]);
