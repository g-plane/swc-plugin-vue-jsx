import { createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
_createVNode(_resolveComponent("C"), {
  "modelValue": foo,
  "modelModifiers": {
    "modifier": true
  },
  "onUpdate:modelValue": $event => foo = $event,
  "bar": bar,
  "barModifiers": {
    "modifier1": true,
    "modifier2": true
  },
  "onUpdate:bar": $event => bar = $event
}, null, 8, ["modelValue", "onUpdate:modelValue", "bar", "onUpdate:bar"]);
