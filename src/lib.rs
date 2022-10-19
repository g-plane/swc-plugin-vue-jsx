use std::collections::HashMap;
use swc_core::{
    common::{Mark, DUMMY_SP},
    ecma::{
        ast::*,
        transforms::testing::test,
        utils::{private_ident, quote_ident, quote_str},
        visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
    },
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};

mod util;

const CREATE_VNODE: &str = "createVNode";
const CREATE_TEXT_VNODE: &str = "createTextVNode";
const FRAGMENT: &str = "Fragment";

#[derive(Default)]
pub struct VueJsxTransformVisitor {
    imports: HashMap<&'static str, Ident>,
    unresolved_mark: Mark,
    slot_helper_ident: Option<Ident>,
}

impl VueJsxTransformVisitor {
    fn import_from_vue(&mut self, item: &'static str) -> Ident {
        self.imports
            .entry(item)
            .or_insert_with_key(|name| private_ident!(*name))
            .clone()
    }

    fn transform_jsx_element(&mut self, jsx_element: &JSXElement) -> Expr {
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(self.import_from_vue(CREATE_VNODE)))),
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
            callee: Callee::Expr(Box::new(Expr::Ident(self.import_from_vue(CREATE_VNODE)))),
            args: vec![
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Ident(self.import_from_vue(FRAGMENT))),
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
            JSXElementName::Ident(ident) => {
                let name = &*ident.sym;
                if name.as_bytes()[0].is_ascii_lowercase()
                    && (css_dataset::tags::STANDARD_HTML_TAGS.contains(name)
                        || css_dataset::tags::SVG_TAGS.contains(name))
                {
                    Expr::Lit(Lit::Str(quote_str!(name)))
                } else if name == FRAGMENT {
                    Expr::Ident(self.import_from_vue(FRAGMENT))
                } else if ident.to_id().1.has_mark(self.unresolved_mark) {
                    // for components that can't be resolved from current file
                    Expr::Call(CallExpr {
                        span: DUMMY_SP,
                        callee: Callee::Expr(Box::new(Expr::Ident(
                            self.import_from_vue("resolveComponent"),
                        ))),
                        args: vec![ExprOrSpread {
                            spread: None,
                            expr: Box::new(Expr::Lit(Lit::Str(quote_str!(name)))),
                        }],
                        type_args: None,
                    })
                } else {
                    Expr::Ident(ident.clone())
                }
            }
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
        let elems = children
            .iter()
            .map(|child| match child {
                JSXElementChild::JSXText(jsx_text) => Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_jsx_text(jsx_text)),
                }),
                JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::JSXEmptyExpr(..),
                    ..
                }) => None,
                JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::Expr(expr),
                    ..
                }) => Some(ExprOrSpread {
                    spread: None,
                    expr: expr.clone(),
                }),
                JSXElementChild::JSXSpreadChild(JSXSpreadChild { expr, .. }) => {
                    Some(ExprOrSpread {
                        spread: Some(DUMMY_SP),
                        expr: expr.clone(),
                    })
                }
                JSXElementChild::JSXElement(jsx_element) => Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_jsx_element(&*jsx_element)),
                }),
                JSXElementChild::JSXFragment(jsx_fragment) => Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_jsx_fragment(jsx_fragment)),
                }),
            })
            .filter_map(|item| item)
            .map(Some)
            .collect::<Vec<_>>();

        if elems.is_empty() {
            Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
        } else {
            Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(quote_ident!("default")),
                    value: Box::new(Expr::Arrow(ArrowExpr {
                        span: DUMMY_SP,
                        params: vec![],
                        body: BlockStmtOrExpr::Expr(Box::new(Expr::Array(ArrayLit {
                            span: DUMMY_SP,
                            elems,
                        }))),
                        is_async: false,
                        is_generator: false,
                        type_params: None,
                        return_type: None,
                    })),
                })))],
            })
        }
    }

    fn transform_jsx_text(&mut self, jsx_text: &JSXText) -> Expr {
        Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(
                self.import_from_vue(CREATE_TEXT_VNODE),
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

        if let Some(slot_helper) = &self.slot_helper_ident {
            module.body.insert(
                0,
                ModuleItem::Stmt(Stmt::Decl(Decl::Fn(util::build_slot_helper(
                    slot_helper.clone(),
                    self.import_from_vue("isVNode"),
                )))),
            )
        }

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
pub fn vue_jsx(program: Program, metadata: TransformPluginProgramMetadata) -> Program {
    program.fold_with(&mut as_folder(VueJsxTransformVisitor {
        unresolved_mark: metadata.unresolved_mark,
        ..Default::default()
    }))
}

test!(
    swc_ecma_parser::Syntax::Es(swc_ecma_parser::EsConfig {
        jsx: true,
        ..Default::default()
    }),
    |_| {
        use swc_core::{common::chain, ecma::transforms::base::resolver};
        let unresolved_mark = Mark::new();
        chain!(
            resolver(unresolved_mark, Mark::new(), false),
            as_folder(VueJsxTransformVisitor {
                unresolved_mark,
                ..Default::default()
            })
        )
    },
    basic,
    r#"const App = <Comp v={afa}>{}{}</Comp>;"#,
    r#""#
);
