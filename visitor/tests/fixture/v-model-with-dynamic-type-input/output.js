import { createVNode as _createVNode, vModelDynamic as _vModelDynamic, withDirectives as _withDirectives } from "vue";
_withDirectives(_createVNode("input", {
  "type": type,
  "onUpdate:modelValue": $event => test = $event
}, null, 8, ["type", "onUpdate:modelValue"]), [[_vModelDynamic, test]]);
