import { Fragment as _Fragment, createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
const foo = 'foo';
const a = () => 'a';
const b = {
  c: 'c'
};
_createVNode(_Fragment, null, [
  _createVNode(_resolveComponent("A"), { [foo]: xx, ["onUpdate" + foo]: $event => xx = $event }, null, 16),
  _createVNode(_resolveComponent("B"), {
    "modelValue": xx,
    "modelModifiers": { "a": true },
    "onUpdate:modelValue": $event => xx = $event,
  }, null, 8, ["modelValue", "onUpdate:modelValue"]),
  _createVNode(_resolveComponent("C"), {
    [foo]: xx,
    [foo + "Modifiers"]: {
      "a": true
    },
    ["onUpdate" + foo]: $event => xx = $event,
  }, null, 16),
  _createVNode(_resolveComponent("D"), {
    [foo === 'foo' ? 'a' : 'b']: xx,
    [(foo === 'foo' ? 'a' : 'b') + "Modifiers"]: {
      "a": true
    },
    ["onUpdate" + (foo === 'foo' ? 'a' : 'b')]: $event => xx = $event,
  }, null, 16),
  _createVNode(_resolveComponent("E"), {
    [a()]: xx,
    [a() + "Modifiers"]: {
      "a": true
    },
    ["onUpdate" + a()]: $event => xx = $event
  }, null, 16),
  _createVNode(_resolveComponent("F"), {
    [b.c]: xx,
    [b.c + "Modifiers"]: {
      "a": true
    },
    ["onUpdate" + b.c]: $event => xx = $event
  }, null, 16),
]);
