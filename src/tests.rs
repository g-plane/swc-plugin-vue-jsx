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
