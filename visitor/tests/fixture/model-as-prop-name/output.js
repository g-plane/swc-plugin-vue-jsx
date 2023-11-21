import { createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
_createVNode(_resolveComponent("C"), {
  "model": foo,
  "onUpdate:model": $event => foo = $event
}, null, 8, ["model", "onUpdate:model"]);
