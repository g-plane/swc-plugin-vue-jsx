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

impl VueJsxTransformVisitor {
    fn transform_jsx_element(&mut self, jsx_element: &JSXElement) -> Expr {
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(
                self.imports
                    .entry("createVNode")
                    .or_insert_with_key(|name| private_ident!(*name))
                    .clone(),
            ))),
            args: vec![
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_tag(&jsx_element.opening.name)),
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(if jsx_element.opening.attrs.is_empty() {
                        Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
                    } else {
                        Expr::Object(self.transform_attrs(&jsx_element.opening.attrs))
                    }),
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_children(&jsx_element.children)),
                },
            ],
            type_args: None,
        })
    }

    fn transform_jsx_fragment(&mut self, jsx_fragment: &JSXFragment) -> Expr {
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(
                self.imports
                    .entry("createVNode")
                    .or_insert_with_key(|name| private_ident!(*name))
                    .clone(),
            ))),
            args: vec![
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Ident(
                        self.imports
                            .entry("Fragment")
                            .or_insert_with_key(|name| private_ident!(*name))
                            .clone(),
                    )),
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))),
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_children(&jsx_fragment.children)),
                },
            ],
            type_args: None,
        })
    }

    fn transform_tag(&mut self, jsx_element_name: &JSXElementName) -> Expr {
        match jsx_element_name {
            JSXElementName::Ident(ident) => Expr::Ident(ident.clone()),
            JSXElementName::JSXMemberExpr(expr) => Expr::JSXMember(expr.clone()),
            JSXElementName::JSXNamespacedName(name) => Expr::JSXNamespacedName(name.clone()),
        }
    }

    fn transform_attrs(&mut self, attrs: &[JSXAttrOrSpread]) -> ObjectLit {
        ObjectLit {
            span: DUMMY_SP,
            props: attrs
                .iter()
                .map(|jsx_attr_or_spread| match jsx_attr_or_spread {
                    JSXAttrOrSpread::JSXAttr(jsx_attr) => {
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: match &jsx_attr.name {
                                JSXAttrName::Ident(ident) => PropName::Str(quote_str!(&ident.sym)),
                                JSXAttrName::JSXNamespacedName(name) => PropName::Str(quote_str!(
                                    format!("{}:{}", name.ns.sym, name.name.sym)
                                )),
                            },
                            value: jsx_attr
                                .value
                                .as_ref()
                                .map(|value| match value {
                                    JSXAttrValue::Lit(lit) => Box::new(Expr::Lit(lit.clone())),
                                    JSXAttrValue::JSXExprContainer(JSXExprContainer {
                                        expr: JSXExpr::Expr(expr),
                                        ..
                                    }) => expr.clone(),
                                    JSXAttrValue::JSXExprContainer(JSXExprContainer {
                                        expr: JSXExpr::JSXEmptyExpr(expr),
                                        ..
                                    }) => Box::new(Expr::JSXEmpty(expr.clone())),
                                    JSXAttrValue::JSXElement(element) => {
                                        Box::new(Expr::JSXElement(element.clone()))
                                    }
                                    JSXAttrValue::JSXFragment(fragment) => {
                                        Box::new(Expr::JSXFragment(fragment.clone()))
                                    }
                                })
                                .unwrap_or_else(|| {
                                    Box::new(Expr::Lit(Lit::Bool(Bool {
                                        span: DUMMY_SP,
                                        value: true,
                                    })))
                                }),
                        })))
                    }
                    JSXAttrOrSpread::SpreadElement(spread) => PropOrSpread::Spread(spread.clone()),
                })
                .collect(),
        }
    }

    fn transform_children(&mut self, children: &[JSXElementChild]) -> Expr {
        if children.is_empty() {
            return Expr::Lit(Lit::Null(Null { span: DUMMY_SP }));
        }

        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("default")),
                value: Box::new(Expr::Arrow(ArrowExpr {
                    span: DUMMY_SP,
                    params: vec![],
                    body: BlockStmtOrExpr::Expr(Box::new(Expr::Array(ArrayLit {
                        span: DUMMY_SP,
                        elems: children
                            .iter()
                            .map(|child| match child {
                                JSXElementChild::JSXText(jsx_text) => ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(self.transform_jsx_text(jsx_text)),
                                },
                                JSXElementChild::JSXExprContainer(JSXExprContainer {
                                    expr: JSXExpr::JSXEmptyExpr(..),
                                    ..
                                }) => todo!(),
                                JSXElementChild::JSXExprContainer(JSXExprContainer {
                                    expr: JSXExpr::Expr(expr),
                                    ..
                                }) => ExprOrSpread {
                                    spread: None,
                                    expr: expr.clone(),
                                },
                                JSXElementChild::JSXSpreadChild(JSXSpreadChild {
                                    expr, ..
                                }) => ExprOrSpread {
                                    spread: Some(DUMMY_SP),
                                    expr: expr.clone(),
                                },
                                JSXElementChild::JSXElement(jsx_element) => ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(self.transform_jsx_element(&*jsx_element)),
                                },
                                JSXElementChild::JSXFragment(jsx_fragment) => ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(self.transform_jsx_fragment(jsx_fragment)),
                                },
                            })
                            .map(Some)
                            .collect(),
                    }))),
                    is_async: false,
                    is_generator: false,
                    type_params: None,
                    return_type: None,
                })),
            })))],
        })
    }

    fn transform_jsx_text(&mut self, jsx_text: &JSXText) -> Expr {
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(
                self.imports
                    .entry("createTextVNode")
                    .or_insert_with_key(|name| private_ident!(*name))
                    .clone(),
            ))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(Lit::Str(quote_str!(&*jsx_text.value)))),
            }],
            type_args: None,
        })
    }
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
        match expr {
            Expr::JSXElement(jsx_element) => *expr = self.transform_jsx_element(jsx_element),
            Expr::JSXFragment(jsx_fragment) => *expr = self.transform_jsx_fragment(jsx_fragment),
            _ => {}
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
    r#"const App = <Comp v={afa}></Comp>;"#,
    r#""#
);
