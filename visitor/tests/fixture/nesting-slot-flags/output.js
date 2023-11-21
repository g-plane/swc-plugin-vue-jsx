import { createVNode as _createVNode, isVNode as _isVNode, resolveComponent as _resolveComponent } from "vue";
function _isSlot(s) {
  return typeof s === "function" || ({}).toString.call(s) === "[object Object]" && !_isVNode(s);
}
let defined;
_createVNode(_resolveComponent("Comp"), null, {
  default: () => [
    unknown1,
    _createVNode(_resolveComponent("Comp"), null, {
      default: () => [unknown2, _createVNode(_resolveComponent("Comp"), null, {
        default: () => [unknown3, _createVNode(_resolveComponent("Comp"), null, _isSlot(defined) ? defined : {
          default: () => [defined],
          _: 2
        })],
        _: 2
      })],
      _: 2
    }),
    _createVNode(_resolveComponent("Comp"), null, {
      default: () => [unknown4, _createVNode(_resolveComponent("Comp"), null, _isSlot(unknown5) ? unknown5 : {
        default: () => [unknown5],
        _: 1
      })],
      _: 1
    }),
  ],
  _: 2
});
