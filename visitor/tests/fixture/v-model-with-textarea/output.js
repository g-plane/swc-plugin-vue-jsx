import { createVNode as _createVNode, vModelText as _vModelText, withDirectives as _withDirectives } from "vue";
_withDirectives(_createVNode("textarea", {
  "onUpdate:modelValue": $event => test = $event
}, null, 8, ["onUpdate:modelValue"]), [[_vModelText, test]]);
