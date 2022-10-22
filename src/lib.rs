use options::Options;
use std::{collections::HashMap, mem};
use swc_core::{
    common::{Mark, Spanned, DUMMY_SP},
    ecma::{
        ast::*,
        transforms::testing::test,
        utils::{private_ident, quote_ident, quote_str},
        visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
    },
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};

mod options;
mod util;

const CREATE_VNODE: &str = "createVNode";
const CREATE_TEXT_VNODE: &str = "createTextVNode";
const FRAGMENT: &str = "Fragment";
const KEEP_ALIVE: &str = "KeepAlive";

#[derive(Default)]
pub struct VueJsxTransformVisitor {
    options: Options,
    imports: HashMap<&'static str, HashMap<&'static str, Ident>>,
    unresolved_mark: Mark,

    slot_helper_ident: Option<Ident>,
    injecting_vars: Vec<VarDeclarator>,
    slot_counter: usize,
}

impl VueJsxTransformVisitor {
    fn import_from_vue(&mut self, item: &'static str) -> Ident {
        self.imports
            .entry("vue")
            .or_default()
            .entry(item)
            .or_insert_with_key(|name| private_ident!(format!("_{name}")))
            .clone()
    }

    fn generate_slot_helper(&mut self) -> Ident {
        self.slot_helper_ident
            .get_or_insert_with(|| private_ident!("_isSlot"))
            .clone()
    }

    fn transform_jsx_element(&mut self, jsx_element: &JSXElement) -> Expr {
        let is_component = self.is_component(&jsx_element.opening.name);
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
                    expr: Box::new(self.transform_attrs(&jsx_element.opening.attrs)),
                },
                ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_children(&jsx_element.children, is_component)),
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
                    expr: Box::new(self.transform_children(&jsx_fragment.children, false)),
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
                } else if self
                    .options
                    .custom_element_patterns
                    .iter()
                    .any(|pattern| pattern.is_match(name))
                {
                    Expr::Lit(Lit::Str(quote_str!(name)))
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

    fn transform_attrs(&mut self, attrs: &[JSXAttrOrSpread]) -> Expr {
        if attrs.is_empty() {
            return Expr::Lit(Lit::Null(Null { span: DUMMY_SP }));
        }

        let props = attrs.iter().fold(
            Vec::with_capacity(attrs.len()),
            |mut props, jsx_attr_or_spread| {
                match jsx_attr_or_spread {
                    JSXAttrOrSpread::JSXAttr(jsx_attr) => {
                        props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
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
                        }))));
                    }
                    JSXAttrOrSpread::SpreadElement(spread) => {
                        if let (Expr::Object(object), false) =
                            (&*spread.expr, self.options.merge_props)
                        {
                            props.extend_from_slice(&object.props);
                        } else {
                            props.push(PropOrSpread::Spread(spread.clone()));
                        }
                    }
                }
                props
            },
        );

        if self.options.merge_props {
            let capacity = props.len();
            let (args, ..) = props.into_iter().fold(
                (Vec::<ExprOrSpread>::with_capacity(capacity), false),
                // The `last_is_spread` flag is used to track whether previous element is "spread element" or not.
                // It's used for handling the case:
                // ```
                // <Component {...{ a: "b" }} c="d" />
                // ```
                // For the example above, the object inside spread element will be picked,
                // so they become two entries:
                // [[a, "b"], [c, "d"]]
                // Without this special logic, those two entries will be merged into one object:
                // `{ a: "b", c: "d" }`
                // which doesn't match the behavior of official Babel plugin,
                // so we need a flag to distinguish them.
                // When handling the attribute `c="d"`, it knows previous element is a spread element,
                // and it will create a new object, instead of reusing previous object which is from that spread element.
                |(mut args, last_is_spread), prop_or_spread| match prop_or_spread {
                    PropOrSpread::Prop(prop) => {
                        // merge current prop to the existing object literal
                        // only when previous element is not a "spread element"
                        if let (Some(Expr::Object(object)), false) =
                            (args.last_mut().map(|arg| &mut *arg.expr), last_is_spread)
                        {
                            object.props.push(PropOrSpread::Prop(prop));
                        } else {
                            args.push(ExprOrSpread {
                                spread: None,
                                expr: Box::new(Expr::Object(ObjectLit {
                                    span: DUMMY_SP,
                                    props: vec![PropOrSpread::Prop(prop)],
                                })),
                            });
                        }
                        (args, false)
                    }
                    PropOrSpread::Spread(SpreadElement { expr, .. }) => {
                        args.push(ExprOrSpread { spread: None, expr });
                        (args, true)
                    }
                },
            );
            match args.first() {
                Some(ExprOrSpread { spread: None, expr }) if args.len() == 1 => *expr.clone(),
                _ => Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Ident(self.import_from_vue("mergeProps")))),
                    args,
                    type_args: None,
                }),
            }
        } else {
            Expr::Object(ObjectLit {
                span: DUMMY_SP,
                props,
            })
        }
    }

    fn transform_children(&mut self, children: &[JSXElementChild], is_component: bool) -> Expr {
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

        match elems.as_slice() {
            [] => Expr::Lit(Lit::Null(Null { span: DUMMY_SP })),
            [Some(ExprOrSpread { spread: None, expr })] => match &**expr {
                expr @ Expr::Ident(..) if is_component => {
                    if self.options.enable_object_slots {
                        Expr::Cond(CondExpr {
                            span: DUMMY_SP,
                            test: Box::new(Expr::Call(CallExpr {
                                span: DUMMY_SP,
                                callee: Callee::Expr(Box::new(Expr::Ident(
                                    self.generate_slot_helper(),
                                ))),
                                args: vec![ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(expr.clone()),
                                }],
                                type_args: None,
                            })),
                            cons: Box::new(expr.clone()),
                            alt: Box::new(self.wrap_children(elems)),
                        })
                    } else {
                        self.wrap_children(elems)
                    }
                }
                expr @ Expr::Call(..) if expr.span() != DUMMY_SP && is_component => {
                    // the element was generated and doesn't have location information
                    if self.options.enable_object_slots {
                        let slot_ident = self.generate_unique_slot_ident();
                        Expr::Cond(CondExpr {
                            span: DUMMY_SP,
                            test: Box::new(Expr::Call(CallExpr {
                                span: DUMMY_SP,
                                callee: Callee::Expr(Box::new(Expr::Ident(
                                    self.generate_slot_helper(),
                                ))),
                                args: vec![ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(Expr::Assign(AssignExpr {
                                        span: DUMMY_SP,
                                        op: op!("="),
                                        left: PatOrExpr::Expr(Box::new(Expr::Ident(
                                            slot_ident.clone(),
                                        ))),
                                        right: Box::new(expr.clone()),
                                    })),
                                }],
                                type_args: None,
                            })),
                            cons: Box::new(Expr::Ident(slot_ident.clone())),
                            alt: Box::new(self.wrap_children(vec![Some(ExprOrSpread {
                                spread: None,
                                expr: Box::new(Expr::Ident(slot_ident)),
                            })])),
                        })
                    } else {
                        self.wrap_children(elems)
                    }
                }
                expr @ Expr::Fn(..) | expr @ Expr::Arrow(..) => Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: PropName::Ident(quote_ident!("default")),
                        value: Box::new(expr.clone()),
                    })))],
                }),
                expr @ Expr::Object(..) => expr.clone(), // TODO: `optimize` option
                _ => {
                    if is_component {
                        self.wrap_children(elems)
                    } else {
                        Expr::Array(ArrayLit {
                            span: DUMMY_SP,
                            elems,
                        })
                    }
                }
            },
            _ => {
                if is_component {
                    self.wrap_children(elems)
                } else {
                    Expr::Array(ArrayLit {
                        span: DUMMY_SP,
                        elems,
                    })
                }
            }
        }
    }

    fn wrap_children(&self, elems: Vec<Option<ExprOrSpread>>) -> Expr {
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

    fn generate_unique_slot_ident(&mut self) -> Ident {
        let ident = if self.slot_counter == 1 {
            private_ident!("_slot")
        } else {
            private_ident!(format!("_slot{}", self.slot_counter))
        };
        self.injecting_vars.push(VarDeclarator {
            span: DUMMY_SP,
            name: Pat::Ident(BindingIdent {
                id: ident.clone(),
                type_ann: None,
            }),
            init: None,
            definite: false,
        });

        self.slot_counter += 1;
        ident
    }

    fn transform_jsx_text(&mut self, jsx_text: &JSXText) -> Expr {
        let jsx_text_value = jsx_text.value.replace('\t', " ");
        let mut jsx_text_lines = jsx_text_value.lines().enumerate().peekable();

        let mut lines = vec![];
        while let Some((index, line)) = jsx_text_lines.next() {
            let line = if index == 0 {
                // first line
                line.trim_end()
            } else if jsx_text_lines.peek().is_none() {
                // last line
                line.trim_start()
            } else {
                line.trim()
            };
            if !line.is_empty() {
                lines.push(line);
            }
        }
        let text = lines.join(" ");
        let lit = if text.is_empty() {
            Lit::Null(Null { span: DUMMY_SP })
        } else {
            Lit::Str(quote_str!(text))
        };

        Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(
                self.import_from_vue(CREATE_TEXT_VNODE),
            ))),
            args: vec![ExprOrSpread {
                spread: None,
                expr: Box::new(Expr::Lit(lit)),
            }],
            type_args: None,
        })
    }

    fn is_component(&self, element_name: &JSXElementName) -> bool {
        let name = match element_name {
            JSXElementName::Ident(Ident { sym, .. }) => &*sym,
            JSXElementName::JSXMemberExpr(JSXMemberExpr { prop, .. }) => &*prop.sym,
            JSXElementName::JSXNamespacedName(JSXNamespacedName { name, .. }) => &*name.sym,
        };
        let should_transformed_to_slots = !self
            .imports
            .get("vue")
            .and_then(|members| members.get(FRAGMENT))
            .map(|ident| &*ident.sym == name)
            .unwrap_or_default()
            && name != KEEP_ALIVE;

        if matches!(element_name, JSXElementName::JSXMemberExpr(..)) {
            should_transformed_to_slots
        } else {
            self.options
                .custom_element_patterns
                .iter()
                .all(|pattern| !pattern.is_match(name))
                && should_transformed_to_slots
                && !(name.as_bytes()[0].is_ascii_lowercase()
                    && (css_dataset::tags::STANDARD_HTML_TAGS.contains(name)
                        || css_dataset::tags::SVG_TAGS.contains(name)))
        }
    }
}

impl VisitMut for VueJsxTransformVisitor {
    fn visit_mut_module(&mut self, module: &mut Module) {
        module.visit_mut_children_with(self);

        if !self.injecting_vars.is_empty() {
            module.body.insert(
                0,
                ModuleItem::Stmt(Stmt::Decl(Decl::Var(Box::new(VarDecl {
                    span: DUMMY_SP,
                    kind: VarDeclKind::Let,
                    declare: false,
                    decls: mem::take(&mut self.injecting_vars),
                })))),
            );
            self.slot_counter = 1;
        }

        if let Some(slot_helper) = &self.slot_helper_ident {
            module.body.insert(
                0,
                ModuleItem::Stmt(Stmt::Decl(Decl::Fn(util::build_slot_helper(
                    slot_helper.clone(),
                    self.import_from_vue("isVNode"),
                )))),
            )
        }

        if let Some(members) = self.imports.get("vue") {
            module.body.insert(
                0,
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                    span: DUMMY_SP,
                    specifiers: members
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
            );
        }
    }

    fn visit_mut_stmts(&mut self, stmts: &mut Vec<Stmt>) {
        stmts.visit_mut_children_with(self);

        if !self.injecting_vars.is_empty() {
            stmts.insert(
                0,
                Stmt::Decl(Decl::Var(Box::new(VarDecl {
                    span: DUMMY_SP,
                    kind: VarDeclKind::Let,
                    declare: false,
                    decls: mem::take(&mut self.injecting_vars),
                }))),
            );
            self.slot_counter = 1;
        }
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
    let options = metadata
        .get_transform_plugin_config()
        .map(|json| {
            serde_json::from_str(&json).expect("failed to parse config of plugin 'vue-jsx'")
        })
        .unwrap_or_default();
    program.fold_with(&mut as_folder(VueJsxTransformVisitor {
        unresolved_mark: metadata.unresolved_mark,
        slot_counter: 1,
        options,
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
                slot_counter: 1,
                options: Options {
                    ..Default::default()
                },
                ..Default::default()
            })
        )
    },
    basic,
    r#"const App = <Comp {...{a:b}} c='d'>{}{}</Comp>;"#,
    r#""#
);
