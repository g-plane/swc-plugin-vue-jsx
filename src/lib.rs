use std::collections::HashMap;
use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::*,
        transforms::testing::test,
        utils::{private_ident, quote_ident, quote_str},
        visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
    },
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};

pub struct VueJsxTransformVisitor {
    imports: HashMap<&'static str, Ident>,
}

impl VisitMut for VueJsxTransformVisitor {
    fn visit_mut_module(&mut self, module: &mut Module) {
        module.visit_mut_children_with(self);

        module.body.insert(
            0,
            ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: self
                    .imports
                    .iter()
                    .map(|(imported, local)| {
                        ImportSpecifier::Named(ImportNamedSpecifier {
                            span: DUMMY_SP,
                            local: local.clone(),
                            imported: Some(ModuleExportName::Ident(quote_ident!(*imported))),
                            is_type_only: false,
                        })
                    })
                    .collect(),
                src: Box::new(quote_str!("vue")),
                type_only: false,
                asserts: None,
            })),
        )
    }

    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        if let Expr::JSXElement(jsx) = expr {
            *expr = Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(Expr::Ident(
                    self.imports
                        .entry("createVNode")
                        .or_insert_with_key(|name| private_ident!(*name))
                        .clone(),
                ))),
                args: vec![match &jsx.opening.name {
                    JSXElementName::Ident(ident) => ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Ident(ident.clone())),
                    },
                    JSXElementName::JSXMemberExpr(expr) => ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::JSXMember(expr.clone())),
                    },
                    JSXElementName::JSXNamespacedName(name) => ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::JSXNamespacedName(name.clone())),
                    },
                }],
                type_args: None,
            });
        }

        expr.visit_mut_children_with(self);
    }
}

#[plugin_transform]
pub fn vue_jsx(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    program.fold_with(&mut as_folder(VueJsxTransformVisitor {
        imports: Default::default(),
    }))
}

test!(
    swc_ecma_parser::Syntax::Es(swc_ecma_parser::EsConfig {
        jsx: true,
        ..Default::default()
    }),
    |_| as_folder(VueJsxTransformVisitor {
        imports: Default::default(),
    }),
    basic,
    r#"const App = <Comp></Comp>;"#,
    r#""#
);
