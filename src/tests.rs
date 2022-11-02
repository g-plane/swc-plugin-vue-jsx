use crate::VueJsxTransformVisitor;
use swc_core::{
    common::{chain, Mark},
    ecma::{
        transforms::{base::resolver, testing::test as low_level_test},
        visit::as_folder,
    },
};
use swc_ecma_parser::{EsConfig, Syntax};

macro_rules! test {
    ($name: ident, $input:literal, $expected:literal) => {
        low_level_test!(
            Syntax::Es(EsConfig {
                jsx: true,
                ..Default::default()
            }),
            |_| {
                let unresolved_mark = Mark::new();
                chain!(
                    resolver(unresolved_mark, Mark::new(), false),
                    as_folder(VueJsxTransformVisitor {
                        unresolved_mark,
                        slot_counter: 1,
                        ..Default::default()
                    })
                )
            },
            $name,
            $input,
            $expected
        );
    };
    ($name: ident, $input:literal, $expected:literal, $options: expr) => {
        low_level_test!(
            Syntax::Es(EsConfig {
                jsx: true,
                ..Default::default()
            }),
            |_| {
                let unresolved_mark = Mark::new();
                chain!(
                    resolver(unresolved_mark, Mark::new(), false),
                    as_folder(VueJsxTransformVisitor {
                        unresolved_mark,
                        slot_counter: 1,
                        options: $options,
                        ..Default::default()
                    })
                )
            },
            $name,
            $input,
            $expected
        );
    };
}

test!(
    v_model_with_checkbox,
    r#"<input type="checkbox" v-model={test} />"#,
    r#"
    import { createVNode as _createVNode,  vModelCheckbox as _vModelCheckbox, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("input", {
        "type": "checkbox",
        "onUpdate:modelValue": $event => test = $event
    }, null), [[_vModelCheckbox, test]]);"#
);

test!(
    v_model_with_radio,
    r#"
    <>
        <input type="radio" value="1" v-model={test} name="test" />
        <input type="radio" value="2" v-model={test} name="test" />
    </>
    "#,
    r#"
    import { Fragment as _Fragment, createVNode as _createVNode, vModelRadio as _vModelRadio, withDirectives as _withDirectives } from "vue";
    _createVNode(_Fragment, null, [_withDirectives(_createVNode("input", {
        "type": "radio",
        "value": "1",
        "onUpdate:modelValue": $event => test = $event,
        "name": "test"
    }, null), [[_vModelRadio, test]]), _withDirectives(_createVNode("input", {
        "type": "radio",
        "value": "2",
        "onUpdate:modelValue": $event => test = $event,
        "name": "test"
    }, null), [[_vModelRadio, test]])]);"#
);

test!(
    v_model_with_select,
    r#"
    <select v-model={test}>
        <option value="1">a</option>
        <option value={2}>b</option>
        <option value={3}>c</option>
    </select>
      "#,
    r#"
    import {
        createTextVNode as _createTextVNode,
        createVNode as _createVNode,
        vModelSelect as _vModelSelect,
        withDirectives as _withDirectives,
    } from "vue";
    _withDirectives(_createVNode("select", {
        "onUpdate:modelValue": $event => test = $event
    }, [_createVNode("option", {
        "value": "1"
    }, [_createTextVNode("a")]), _createVNode("option", {
        "value": 2
    }, [_createTextVNode("b")]), _createVNode("option", {
        "value": 3
    }, [_createTextVNode("c")])]), [[_vModelSelect, test]]);"#
);

test!(
    v_model_with_textarea,
    "<textarea v-model={test} />",
    r#"
    import { createVNode as _createVNode, vModelText as _vModelText, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("textarea", {
        "onUpdate:modelValue": $event => test = $event
    }, null), [[_vModelText, test]]);"#
);

test!(
    v_model_with_text_input,
    "<input v-model={test} />",
    r#"
    import { createVNode as _createVNode, vModelText as _vModelText, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("input", {
        "onUpdate:modelValue": $event => test = $event
    }, null), [[_vModelText, test]]);"#
);

test!(
    v_model_with_dynamic_type_input,
    "<input type={type} v-model={test} />",
    r#"
    import { createVNode as _createVNode, vModelDynamic as _vModelDynamic, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("input", {
        "type": type,
        "onUpdate:modelValue": $event => test = $event
    }, null), [[_vModelDynamic, test]]);"#
);

test!(
    v_show,
    "<div v-show={x}>vShow</div>",
    r#"
    import {
        createTextVNode as _createTextVNode,
        createVNode as _createVNode,
        vShow as _vShow,
        withDirectives as _withDirectives,
    } from "vue";
    _withDirectives(_createVNode("div", null, [_createTextVNode("vShow")]), [[_vShow, x]]);
"#
);

test!(
    v_model_with_input_lazy_modifier,
    "<input v-model={[test, ['lazy']]} />",
    r#"
    import { createVNode as _createVNode, vModelText as _vModelText, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("input", {
        "onUpdate:modelValue": $event => test = $event
    }, null), [[_vModelText, test, void 0, {
        lazy: true
    }]]);"#
);

test!(
    custom_directive,
    "<A vCus={x} />",
    r#"
    import {
        createVNode as _createVNode,
        resolveComponent as _resolveComponent,
        resolveDirective as _resolveDirective,
        withDirectives as _withDirectives,
    } from "vue";
    _withDirectives(_createVNode(_resolveComponent("A"), null, null), [[_resolveDirective("cus"), x]]);
"#
);

test!(
    v_html,
    r#"<h1 v-html="<div>foo</div>"></h1>"#,
    r#"
    import { createVNode as _createVNode } from "vue";
    _createVNode("h1", {
        "innerHTML": "<div>foo</div>"
    }, null);
"#
);

test!(
    v_text,
    "<div v-text={text}></div>",
    r#"
    import { createVNode as _createVNode } from "vue";
    _createVNode("div", {
        "textContent": text
    }, null);
"#
);

test!(
    without_props,
    "<a>a</a>",
    r#"
    import { createTextVNode as _createTextVNode, createVNode as _createVNode } from "vue";
    _createVNode("a", null, [_createTextVNode("a")]);
"#
);

test!(
    merge_props_order,
    r#"<button loading {...x} type="submit">btn</button>"#,
    r#"
    import { createTextVNode as _createTextVNode, createVNode as _createVNode, mergeProps as _mergeProps } from "vue";
    _createVNode("button", _mergeProps({
        "loading": true
    }, x, {
        "type": "submit"
    }), [_createTextVNode("btn")]);
"#
);
