use crate::{Options, VueJsxTransformVisitor};
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
            |tester| {
                let unresolved_mark = Mark::new();
                chain!(
                    resolver(unresolved_mark, Mark::new(), false),
                    as_folder(VueJsxTransformVisitor::new(
                        Options {
                            optimize: true,
                            ..Default::default()
                        },
                        unresolved_mark,
                        Some(tester.comments.clone())
                    ))
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
            |tester| {
                let unresolved_mark = Mark::new();
                chain!(
                    resolver(unresolved_mark, Mark::new(), false),
                    as_folder(VueJsxTransformVisitor::new(
                        $options,
                        unresolved_mark,
                        Some(tester.comments.clone())
                    ))
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
    }, null, 8, ["onUpdate:modelValue"]), [[_vModelCheckbox, test]]);"#
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
    }, null, 8, ["onUpdate:modelValue"]), [[_vModelRadio, test]]), _withDirectives(_createVNode("input", {
        "type": "radio",
        "value": "2",
        "onUpdate:modelValue": $event => test = $event,
        "name": "test"
    }, null, 8, ["onUpdate:modelValue"]), [[_vModelRadio, test]])]);"#
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
    }, [_createTextVNode("c")])], 8, ["onUpdate:modelValue"]), [[_vModelSelect, test]]);"#
);

test!(
    v_model_with_textarea,
    "<textarea v-model={test} />",
    r#"
    import { createVNode as _createVNode, vModelText as _vModelText, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("textarea", {
        "onUpdate:modelValue": $event => test = $event
    }, null, 8, ["onUpdate:modelValue"]), [[_vModelText, test]]);"#
);

test!(
    v_model_with_text_input,
    "<input v-model={test} />",
    r#"
    import { createVNode as _createVNode, vModelText as _vModelText, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("input", {
        "onUpdate:modelValue": $event => test = $event
    }, null, 8, ["onUpdate:modelValue"]), [[_vModelText, test]]);"#
);

test!(
    v_model_with_dynamic_type_input,
    "<input type={type} v-model={test} />",
    r#"
    import { createVNode as _createVNode, vModelDynamic as _vModelDynamic, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("input", {
        "type": type,
        "onUpdate:modelValue": $event => test = $event
    }, null, 8, ["onUpdate:modelValue", "type"]), [[_vModelDynamic, test]]);"#
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
    _withDirectives(_createVNode("div", null, [_createTextVNode("vShow")], 512), [[_vShow, x]]);
"#
);

test!(
    v_model_with_input_lazy_modifier,
    "<input v-model={[test, ['lazy']]} />",
    r#"
    import { createVNode as _createVNode, vModelText as _vModelText, withDirectives as _withDirectives } from "vue";
    _withDirectives(_createVNode("input", {
        "onUpdate:modelValue": $event => test = $event
    }, null, 8, ["onUpdate:modelValue"]), [[_vModelText, test, void 0, {
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
    _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("cus"), x]]);
"#
);

test!(
    v_html,
    r#"<h1 v-html="<div>foo</div>"></h1>"#,
    r#"
    import { createVNode as _createVNode } from "vue";
    _createVNode("h1", {
        "innerHTML": "<div>foo</div>"
    }, null, 8, ["innerHTML"]);
"#
);

test!(
    v_text,
    "<div v-text={text}></div>",
    r#"
    import { createVNode as _createVNode } from "vue";
    _createVNode("div", {
        "textContent": text
    }, null, 8, ["textContent"]);
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
    }), [_createTextVNode("btn")], 16, ["loading"]);
"#
);

test!(
    single_attr,
    "<div {...x}>single</div>",
    r#"
    import { createTextVNode as _createTextVNode, createVNode as _createVNode } from "vue";
    _createVNode("div", x, [_createTextVNode("single")], 16);
"#
);

test!(
    keep_namespace_import,
    r#"
    import * as Vue from 'vue';
    <div>Vue</div>"#,
    r#"
    import { createTextVNode as _createTextVNode, createVNode as _createVNode } from "vue";
    import * as Vue from 'vue';
    _createVNode("div", null, [_createTextVNode("Vue")]);
"#
);

test!(
    specifiers_merged_into_single_import_decl,
    r#"
    import { createVNode, Fragment as _Fragment } from 'vue';
    import { vShow } from 'vue'

    <_Fragment />
    "#,
    r#"
    import { createVNode as _createVNode } from "vue";
    import { createVNode, Fragment as _Fragment } from 'vue';
    import { vShow } from 'vue';
    _createVNode(_Fragment, null, null);
    "#
);

test!(
    without_jsx,
    r#"
    import { createVNode } from 'vue';
    createVNode('div', null, ['Without JSX should work']);"#,
    r#"
    import { createVNode } from 'vue';
    createVNode('div', null, ['Without JSX should work']);
"#
);

test!(
    custom_directive_with_argument_and_modifiers,
    r#"
    <>
        <A v-xxx={x} />
        <A v-xxx={[x]} />
        <A v-xxx={[x, 'y']} />
        <A v-xxx={[x, 'y', ['a', 'b']]} />
        <A v-xxx={[x, ['a', 'b']]} />
        <A v-xxx={[x, y, ['a', 'b']]} />
        <A v-xxx={[x, y, ['a', 'b']]} />
      </>"#,
    r#"
    import {
        Fragment as _Fragment,
        createVNode as _createVNode,
        resolveComponent as _resolveComponent,
        resolveDirective as _resolveDirective,
        withDirectives as _withDirectives,
    } from "vue";
    _createVNode(_Fragment, null, [
        _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x]]),
        _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x]]),
        _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, 'y']]),
        _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, 'y', {
            a: true,
            b: true
        }]]),
        _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, void 0, {
            a: true,
            b: true
        }]]),
        _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, y, {
            a: true,
            b: true
        }]]),
        _withDirectives(_createVNode(_resolveComponent("A"), null, null, 512), [[_resolveDirective("xxx"), x, y, {
            a: true,
            b: true
        }]]),
    ]);
"#
);

test!(
    model_as_prop_name,
    r#"<C v-model={[foo, "model"]} />"#,
    r#"
    import { createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
    _createVNode(_resolveComponent("C"), {
        "model": foo,
        "onUpdate:model": $event => foo = $event
    }, null, 8, ["model", "onUpdate:model"]);"#
);

test!(
    keep_alive_named_import,
    r#"
    import { KeepAlive } from 'vue';

    <KeepAlive>123</KeepAlive>
    "#,
    r#"
    import { createTextVNode as _createTextVNode, createVNode as _createVNode } from "vue";
    import { KeepAlive } from 'vue';
    _createVNode(KeepAlive, null, [_createTextVNode("123")]);
    "#
);

test!(
    keep_alive_namespace_import,
    r#"
    import * as Vue from 'vue';

    <Vue.KeepAlive>123</Vue.KeepAlive>
    "#,
    r#"
    import { createTextVNode as _createTextVNode, createVNode as _createVNode } from "vue";
    import * as Vue from 'vue';
    _createVNode(Vue.KeepAlive, null, [_createTextVNode("123")]);
    "#
);

test!(
    custom_pragma_in_comment,
    r#"
    /* @jsx custom */
    <div id="custom">Hello</div>"#,
    r#"
    import { createTextVNode as _createTextVNode } from "vue";
    /* @jsx custom */
    custom("div", { "id": "custom" }, [_createTextVNode("Hello")]);
    "#
);

test!(
    v_model_value_supports_variable,
    "
    const foo = 'foo';

    const a = () => 'a';

    const b = { c: 'c' };
    <>
        <A v-model={[xx, foo]} />
        <B v-model={[xx, ['a']]} />
        <C v-model={[xx, foo, ['a']]} />
        <D v-model={[xx, foo === 'foo' ? 'a' : 'b', ['a']]} />
        <E v-model={[xx, a(), ['a']]} />
        <F v-model={[xx, b.c, ['a']]} />
    </>
",
    r#"
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
    "#
);

test!(
    override_props_single,
    "<div {...a} />",
    r#"
    import { createVNode as _createVNode } from "vue";
    _createVNode("div", a, null);
    "#,
    Options {
        merge_props: false,
        ..Default::default()
    }
);

test!(
    override_props_multiple,
    r#"<A loading {...a} {...{ b: 1, c: { d: 2 } }} class="x" style={x} />"#,
    r#"
    import { createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
    _createVNode(_resolveComponent("A"), {
        "loading": true,
        ...a,
        b: 1,
        c: {
            d: 2
        },
        "class": "x",
        "style": x
    }, null);
    "#,
    Options {
        merge_props: false,
        ..Default::default()
    }
);

test!(
    multiple_exprs_slot,
    "<A>{foo}{bar}</A>",
    r#"
    import { createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
    _createVNode(_resolveComponent("A"), null, {
        default: () => [foo, bar],
        _: 1
    });
    "#,
    Options {
        optimize: true,
        enable_object_slots: true,
        ..Default::default()
    }
);

test!(
    function_expr_slot,
    r#"<A>{() => "foo"}</A>"#,
    r#"
    import { createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
    _createVNode(_resolveComponent("A"), null, {
        default: () => "foo"
    });
    "#,
    Options {
        optimize: true,
        enable_object_slots: true,
        ..Default::default()
    }
);

test!(
    non_literal_expr_slot,
    r#"
    const foo = () => 1;
    <A>{foo()}</A>;
    "#,
    r#"
    import { createVNode as _createVNode, isVNode as _isVNode, resolveComponent as _resolveComponent } from "vue";
    function _isSlot(s) {
        return typeof s === "function" || ({}).toString.call(s) === "[object Object]" && !_isVNode(s);
    }
    let _slot;
    const foo = () => 1;
    _createVNode(_resolveComponent("A"), null, _isSlot(_slot = foo()) ? _slot : {
        default: () => [_slot],
        _: 1
    });
    "#,
    Options {
        optimize: true,
        enable_object_slots: true,
        ..Default::default()
    }
);

test!(
    disable_object_slot,
    "<Badge>{slots.default()}</Badge>",
    r#"
    import { createVNode as _createVNode, resolveComponent as _resolveComponent } from "vue";
    _createVNode(_resolveComponent("Badge"), null, {
        default: () => [slots.default()],
        _: 1
    });
    "#,
    Options {
        optimize: true,
        enable_object_slots: false,
        ..Default::default()
    }
);

test!(
    custom_pragma_in_options,
    "<div>pragma</div>",
    r#"
    import { createTextVNode as _createTextVNode } from "vue";
    custom("div", null, [_createTextVNode("pragma")]);
    "#,
    Options {
        pragma: Some("custom".into()),
        ..Default::default()
    }
);

test!(
    custom_element,
    "<foo><span>foo</span></foo>",
    r#"
    import { createTextVNode as _createTextVNode, createVNode as _createVNode } from "vue";
    _createVNode("foo", null, [_createVNode("span", null, [_createTextVNode("foo")])]);"#,
    Options {
        custom_element_patterns: vec![crate::options::Regex::new("foo").unwrap()],
        ..Default::default()
    }
);

test!(
    fragment_already_imported,
    r#"
    import { Fragment as _Fragment } from 'vue'
    const Root1 = () => <>root1</>
    const Root2 = () => <_Fragment>root2</_Fragment>
    "#,
    r#"
    import { Fragment as _Fragment, createTextVNode as _createTextVNode, createVNode as _createVNode } from "vue";
    import { Fragment as _Fragment1 } from 'vue';
    const Root1 = () => _createVNode(_Fragment, null, [_createTextVNode("root1")]);
    const Root2 = () => _createVNode(_Fragment1, null, [_createTextVNode("root2")]);
    "#,
    Options::default()
);
