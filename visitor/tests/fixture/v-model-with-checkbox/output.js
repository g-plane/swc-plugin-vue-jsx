import { createVNode as _createVNode,  vModelCheckbox as _vModelCheckbox, withDirectives as _withDirectives } from "vue";
_withDirectives(_createVNode("input", {
    "type": "checkbox",
    "onUpdate:modelValue": $event => test = $event
}, null, 8, ["onUpdate:modelValue"]), [[_vModelCheckbox, test]]);
