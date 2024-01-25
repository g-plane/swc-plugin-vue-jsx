import { createVNode as _createVNode, isVNode as _isVNode, resolveComponent as _resolveComponent } from "vue";
function _isSlot(s) {
    return typeof s === "function" || ({}).toString.call(s) === "[object Object]" && !_isVNode(s);
}
const Component = (row)=>{
    let _slot, _slot2, _slot3;
    return _createVNode(_resolveComponent("NSpace"), null, {
        default: ()=>[
                _createVNode(_resolveComponent("NButton"), {
                    "type": "primary",
                    "secondary": true,
                    "onClick": handler1
                }, _isSlot(_slot = t('text1')) ? _slot : {
                    default: ()=>[
                            _slot
                        ],
                    _: 1
                }, 8, [
                    "secondary",
                    "onClick"
                ]),
                _createVNode(_resolveComponent("NButton"), {
                    "onClick": handler2
                }, _isSlot(_slot2 = t('text2')) ? _slot2 : {
                    default: ()=>[
                            _slot2
                        ],
                    _: 1
                }, 8, [
                    "onClick"
                ]),
                _createVNode(_resolveComponent("NButton"), {
                    "type": "error",
                    "onClick": handler3
                }, _isSlot(_slot3 = t('text3')) ? _slot3 : {
                    default: ()=>[
                            _slot3
                        ],
                    _: 1
                }, 8, [
                    "onClick"
                ])
            ],
        _: 1
    });
};
