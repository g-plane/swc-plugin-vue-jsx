#![allow(clippy::not_unsafe_ptr_arg_deref)]

use swc_core::{
    ecma::{
        ast::Program,
        visit::{as_folder, FoldWith},
    },
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};
use swc_vue_jsx_visitor::VueJsxTransformVisitor;

#[plugin_transform]
pub fn vue_jsx(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    let options = metadata
        .get_transform_plugin_config()
        .map(|json| {
            serde_json::from_str(&json).expect("failed to parse config of plugin 'vue-jsx'")
        })
        .unwrap_or_default();
    program.fold_with(&mut as_folder(VueJsxTransformVisitor::new(
        options,
        metadata.unresolved_mark,
        metadata.comments,
    )))
}
