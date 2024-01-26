import { createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
_createVNode(_resolveComponent("Child"), {
    "value": this.foo,
    "valueModifiers": {
        "double": true
    },
    "onUpdate:value": ($event)=>this.foo = $event
}, null, 8, [
    "value",
    "onUpdate:value"
]);
