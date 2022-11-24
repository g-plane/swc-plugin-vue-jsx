use directive::{is_directive, parse_directive, Directive, NormalDirective};
use options::Options;
use patch_flags::PatchFlags;
use slot_flag::SlotFlag;
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    mem,
};
use swc_core::{
    common::{comments::Comments, Mark, Span, Spanned, DUMMY_SP},
    ecma::{
        ast::*,
        atoms::JsWord,
        utils::{private_ident, quote_ident, quote_str},
        visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
    },
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};

mod directive;
mod options;
mod patch_flags;
mod slot_flag;
#[cfg(test)]
mod tests;
mod util;

const FRAGMENT: &str = "Fragment";
const KEEP_ALIVE: &str = "KeepAlive";

pub struct VueJsxTransformVisitor<C>
where
    C: Comments,
{
    options: Options,
    vue_imports: BTreeMap<&'static str, Ident>,
    transform_on_helper: Option<Ident>,

    unresolved_mark: Mark,
    comments: Option<C>,

    pragma: Option<String>,
    slot_helper_ident: Option<Ident>,
    injecting_vars: Vec<VarDeclarator>,
    slot_counter: usize,
    slot_flag_stack: Vec<SlotFlag>,
}

impl<C> VueJsxTransformVisitor<C>
where
    C: Comments,
{
    pub fn new(options: Options, unresolved_mark: Mark, comments: Option<C>) -> Self {
        Self {
            options,
            vue_imports: Default::default(),
            transform_on_helper: None,

            unresolved_mark,
            comments,

            pragma: None,
            slot_helper_ident: None,
            injecting_vars: Default::default(),
            slot_counter: 1,
            slot_flag_stack: Default::default(),
        }
    }

    fn import_from_vue(&mut self, item: &'static str) -> Ident {
        self.vue_imports
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
        self.slot_flag_stack.push(SlotFlag::Stable);

        let is_component = self.is_component(&jsx_element.opening.name);
        let mut directives = vec![];
        let (attributes, patch_flags, dynamic_props) =
            self.transform_attrs(&jsx_element.opening.attrs, is_component, &mut directives);
        let mut vnode_call_args = vec![
            ExprOrSpread {
                spread: None,
                expr: Box::new(self.transform_tag(&jsx_element.opening.name)),
            },
            ExprOrSpread {
                spread: None,
                expr: Box::new(attributes),
            },
            ExprOrSpread {
                spread: None,
                expr: Box::new(self.transform_children(&jsx_element.children, is_component)),
            },
        ];
        if self.options.optimize {
            if !patch_flags.is_empty() {
                vnode_call_args.push(ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Num(Number {
                        span: DUMMY_SP,
                        value: patch_flags.bits() as f64,
                        raw: None,
                    }))),
                });
            }
            match dynamic_props {
                Some(dynamic_props) if !dynamic_props.is_empty() => {
                    vnode_call_args.push(ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Array(ArrayLit {
                            span: DUMMY_SP,
                            elems: dynamic_props
                                .into_iter()
                                .map(|prop| {
                                    Some(ExprOrSpread {
                                        spread: None,
                                        expr: Box::new(Expr::Lit(Lit::Str(quote_str!(prop)))),
                                    })
                                })
                                .collect(),
                        })),
                    })
                }
                _ => {}
            }
        }

        let create_vnode_call = Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(self.get_pragma()))),
            args: vnode_call_args,
            type_args: None,
        });

        if directives.is_empty() {
            create_vnode_call
        } else {
            Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(Expr::Ident(
                    self.import_from_vue("withDirectives"),
                ))),
                args: vec![
                    ExprOrSpread {
                        spread: None,
                        expr: Box::new(create_vnode_call),
                    },
                    ExprOrSpread {
                        spread: None,
                        expr: Box::new(Expr::Array(ArrayLit {
                            span: DUMMY_SP,
                            elems: directives
                                .into_iter()
                                .map(|directive| {
                                    let mut elems =
                                        vec![
                                            Some(ExprOrSpread {
                                                spread: None,
                                                expr: Box::new(self.resolve_directive(
                                                    &directive.name,
                                                    jsx_element,
                                                )),
                                            }),
                                            Some(ExprOrSpread {
                                                spread: None,
                                                expr: Box::new(directive.value),
                                            }),
                                        ];
                                    if let Some(argument) = directive.argument {
                                        elems.push(Some(ExprOrSpread {
                                            spread: None,
                                            expr: Box::new(argument),
                                        }));
                                    }
                                    if let Some(modifiers) = directive.modifiers {
                                        elems.push(Some(ExprOrSpread {
                                            spread: None,
                                            expr: Box::new(modifiers),
                                        }));
                                    }
                                    Some(ExprOrSpread {
                                        spread: None,
                                        expr: Box::new(Expr::Array(ArrayLit {
                                            span: DUMMY_SP,
                                            elems,
                                        })),
                                    })
                                })
                                .collect(),
                        })),
                    },
                ],
                type_args: None,
            })
        }
    }

    fn transform_jsx_fragment(&mut self, jsx_fragment: &JSXFragment) -> Expr {
        self.slot_flag_stack.push(SlotFlag::Stable);

        Expr::Call(CallExpr {
            span: DUMMY_SP,
            callee: Callee::Expr(Box::new(Expr::Ident(self.get_pragma()))),
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

    fn transform_attrs<'a>(
        &mut self,
        attrs: &'a [JSXAttrOrSpread],
        is_component: bool,
        directives: &mut Vec<NormalDirective>,
    ) -> (Expr, PatchFlags, Option<BTreeSet<Cow<'a, str>>>) {
        if attrs.is_empty() {
            return (
                Expr::Lit(Lit::Null(Null { span: DUMMY_SP })),
                PatchFlags::empty(),
                None,
            );
        }

        let mut dynamic_props = BTreeSet::new();

        // patch flags analysis
        let mut has_ref = false;
        let mut has_class_binding = false;
        let mut has_style_binding = false;
        let mut has_hydration_event_binding = false;
        let mut has_dynamic_keys = false;

        let (mut props, mut merge_args) = attrs.iter().fold(
            (
                Vec::with_capacity(attrs.len()),
                Vec::with_capacity(attrs.len()),
            ),
            |(mut props, mut merge_args), jsx_attr_or_spread| {
                match jsx_attr_or_spread {
                    JSXAttrOrSpread::JSXAttr(jsx_attr) if is_directive(jsx_attr) => {
                        match parse_directive(jsx_attr, is_component) {
                            Directive::Normal(directive) => directives.push(directive),
                            Directive::Html(expr) => {
                                props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                    KeyValueProp {
                                        key: PropName::Str(quote_str!("innerHTML")),
                                        value: Box::new(expr),
                                    },
                                ))));
                                dynamic_props.insert("innerHTML".into());
                            }
                            Directive::Text(expr) => {
                                props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                    KeyValueProp {
                                        key: PropName::Str(quote_str!("textContent")),
                                        value: Box::new(expr),
                                    },
                                ))));
                                dynamic_props.insert("textContent".into());
                            }
                            Directive::VModel(directive) => {
                                if is_component {
                                    props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                        KeyValueProp {
                                            key: match &directive.argument {
                                                Some(Expr::Lit(Lit::Null(..))) | None => {
                                                    dynamic_props.insert("modelValue".into());
                                                    PropName::Str(quote_str!("modelValue"))
                                                }
                                                Some(Expr::Lit(Lit::Str(Str {
                                                    value, ..
                                                }))) => {
                                                    dynamic_props
                                                        .insert(Cow::from(value.to_string()));
                                                    PropName::Str(quote_str!(value))
                                                }
                                                Some(expr) => {
                                                    PropName::Computed(ComputedPropName {
                                                        span: DUMMY_SP,
                                                        expr: Box::new(expr.clone()),
                                                    })
                                                }
                                            },
                                            value: Box::new(directive.value.clone()),
                                        },
                                    ))));
                                    if let Some(modifiers) = directive.modifiers {
                                        props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                            KeyValueProp {
                                                key: match &directive.argument {
                                                    Some(Expr::Lit(Lit::Null(..))) | None => {
                                                        PropName::Str(quote_str!("modelModifiers"))
                                                    }
                                                    Some(Expr::Lit(Lit::Str(Str {
                                                        value,
                                                        ..
                                                    }))) => PropName::Str(quote_str!(format!(
                                                        "{value}Modifiers"
                                                    ))),
                                                    Some(expr) => {
                                                        PropName::Computed(ComputedPropName {
                                                            span: DUMMY_SP,
                                                            expr: Box::new(Expr::Bin(BinExpr {
                                                                span: DUMMY_SP,
                                                                op: op!(bin, "+"),
                                                                left: Box::new(expr.clone()),
                                                                right: Box::new(Expr::Lit(
                                                                    Lit::Str(quote_str!(
                                                                        "Modifiers"
                                                                    )),
                                                                )),
                                                            })),
                                                        })
                                                    }
                                                },
                                                value: Box::new(modifiers),
                                            },
                                        ))))
                                    }
                                } else {
                                    directives.push(NormalDirective {
                                        name: JsWord::from("model"),
                                        argument: directive.transformed_argument,
                                        modifiers: directive.modifiers.clone(),
                                        value: directive.value.clone(),
                                    });
                                }

                                props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                    KeyValueProp {
                                        key: match directive.argument {
                                            Some(Expr::Lit(Lit::Null(..))) | None => {
                                                dynamic_props.insert("onUpdate:modelValue".into());
                                                PropName::Str(quote_str!("onUpdate:modelValue"))
                                            }
                                            Some(Expr::Lit(Lit::Str(Str { value, .. }))) => {
                                                let name = format!("onUpdate:{value}");
                                                let prop_name = PropName::Str(quote_str!(&*name));
                                                dynamic_props.insert(name.into());
                                                prop_name
                                            }
                                            Some(expr) => {
                                                has_dynamic_keys = true;
                                                PropName::Computed(ComputedPropName {
                                                    span: DUMMY_SP,
                                                    expr: Box::new(Expr::Bin(BinExpr {
                                                        span: DUMMY_SP,
                                                        op: op!(bin, "+"),
                                                        left: Box::new(Expr::Lit(Lit::Str(
                                                            quote_str!("onUpdate"),
                                                        ))),
                                                        right: Box::new(expr),
                                                    })),
                                                })
                                            }
                                        },
                                        value: Box::new(Expr::Arrow(ArrowExpr {
                                            span: DUMMY_SP,
                                            params: vec![Pat::Ident(BindingIdent {
                                                id: quote_ident!("$event"),
                                                type_ann: None,
                                            })],
                                            body: BlockStmtOrExpr::Expr(Box::new(Expr::Assign(
                                                AssignExpr {
                                                    span: DUMMY_SP,
                                                    op: op!("="),
                                                    left: PatOrExpr::Expr(Box::new(
                                                        directive.value,
                                                    )),
                                                    right: Box::new(Expr::Ident(quote_ident!(
                                                        "$event"
                                                    ))),
                                                },
                                            ))),
                                            is_async: false,
                                            is_generator: false,
                                            type_params: None,
                                            return_type: None,
                                        })),
                                    },
                                ))));
                            }
                        }
                    }
                    JSXAttrOrSpread::JSXAttr(jsx_attr) => {
                        let attr_name = match &jsx_attr.name {
                            JSXAttrName::Ident(ident) => Cow::from(&*ident.sym),
                            JSXAttrName::JSXNamespacedName(name) => {
                                Cow::from(format!("{}:{}", name.ns.sym, name.name.sym))
                            }
                        };
                        let attr_value = jsx_attr
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
                                }) => Box::new(Expr::JSXEmpty(*expr)),
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
                            });

                        if attr_name == "ref" {
                            has_ref = true;
                        } else if !jsx_attr
                            .value
                            .as_ref()
                            .map(util::is_jsx_attr_value_constant)
                            .unwrap_or_default()
                        {
                            if !is_component && util::is_on(&attr_name)
                                // omit the flag for click handlers becaues hydration gives click
                                // dedicated fast path.
                                && !attr_name.eq_ignore_ascii_case("onclick")
                                // omit v-model handlers
                                && attr_name != "onUpdate:modelValue"
                            {
                                has_hydration_event_binding = true;
                            }
                            match &*attr_name {
                                "class" if !is_component => has_class_binding = true,
                                "style" if !is_component => has_style_binding = true,
                                "key" | "on" | "ref" => {}
                                _ => {
                                    dynamic_props.insert(attr_name.clone());
                                }
                            }
                        }

                        if self.options.transform_on
                            && (attr_name == "on" || attr_name == "nativeOn")
                        {
                            merge_args.push(Expr::Call(CallExpr {
                                span: DUMMY_SP,
                                callee: Callee::Expr(Box::new(Expr::Ident(
                                    self.transform_on_helper
                                        .get_or_insert_with(|| private_ident!("_transformOn"))
                                        .clone(),
                                ))),
                                args: vec![ExprOrSpread {
                                    spread: None,
                                    expr: attr_value,
                                }],
                                type_args: None,
                            }));
                        } else {
                            props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(
                                KeyValueProp {
                                    key: PropName::Str(quote_str!(attr_name)),
                                    value: attr_value,
                                },
                            ))));
                        }
                    }
                    JSXAttrOrSpread::SpreadElement(spread) => {
                        has_dynamic_keys = true;

                        if !props.is_empty() && self.options.merge_props {
                            merge_args.push(Expr::Object(ObjectLit {
                                span: DUMMY_SP,
                                props: mem::take(&mut props),
                            }));
                        }

                        if let Expr::Object(object) = &*spread.expr {
                            if self.options.merge_props {
                                merge_args.push(Expr::Object(object.clone()));
                            } else {
                                props.extend_from_slice(&object.props);
                            }
                        } else if self.options.merge_props {
                            merge_args.push(*spread.expr.clone());
                        } else {
                            props.push(PropOrSpread::Spread(spread.clone()));
                        }
                    }
                }
                (props, merge_args)
            },
        );

        let expr = if !merge_args.is_empty() {
            if !props.is_empty() {
                merge_args.push(Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: mem::take(&mut props),
                }));
            }
            match merge_args.as_slice() {
                [expr] => expr.clone(),
                _ => Expr::Call(CallExpr {
                    span: DUMMY_SP,
                    callee: Callee::Expr(Box::new(Expr::Ident(self.import_from_vue("mergeProps")))),
                    args: merge_args
                        .into_iter()
                        .map(|expr| ExprOrSpread {
                            spread: None,
                            expr: Box::new(expr),
                        })
                        .collect(),
                    type_args: None,
                }),
            }
        } else if !props.is_empty() {
            if let [PropOrSpread::Spread(SpreadElement { expr, .. })] = props.as_slice() {
                *expr.clone()
            } else {
                Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props,
                })
            }
        } else {
            Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
        };

        let mut patch_flags = PatchFlags::empty();
        if has_dynamic_keys {
            patch_flags.insert(PatchFlags::FULL_PROPS);
        } else {
            if has_class_binding {
                patch_flags.insert(PatchFlags::CLASS);
            }
            if has_style_binding {
                patch_flags.insert(PatchFlags::STYLE);
            }
            if !dynamic_props.is_empty() {
                patch_flags.insert(PatchFlags::PROPS);
            }
            if has_hydration_event_binding {
                patch_flags.insert(PatchFlags::HYDRATE_EVENTS);
            }
        }
        if (patch_flags.is_empty() || patch_flags == PatchFlags::HYDRATE_EVENTS)
            && (has_ref || !directives.is_empty())
        {
            patch_flags.insert(PatchFlags::NEED_PATCH);
        }

        (expr, patch_flags, Some(dynamic_props))
    }

    fn transform_children(&mut self, children: &[JSXElementChild], is_component: bool) -> Expr {
        let elems = children
            .iter()
            .filter_map(|child| match child {
                JSXElementChild::JSXText(jsx_text) => {
                    self.transform_jsx_text(jsx_text).map(|expr| ExprOrSpread {
                        spread: None,
                        expr: Box::new(expr),
                    })
                }
                JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::JSXEmptyExpr(..),
                    ..
                }) => None,
                JSXElementChild::JSXExprContainer(JSXExprContainer {
                    expr: JSXExpr::Expr(expr),
                    ..
                }) => {
                    if let Expr::Ident(ident) = &**expr {
                        if !ident.to_id().1.has_mark(self.unresolved_mark) {
                            self.slot_flag_stack.fill(SlotFlag::Dynamic);
                        }
                    }
                    Some(ExprOrSpread {
                        spread: None,
                        expr: expr.clone(),
                    })
                }
                JSXElementChild::JSXSpreadChild(JSXSpreadChild { expr, .. }) => {
                    if let Expr::Ident(ident) = &**expr {
                        if !ident.to_id().1.has_mark(self.unresolved_mark) {
                            self.slot_flag_stack.fill(SlotFlag::Dynamic);
                        }
                    }
                    Some(ExprOrSpread {
                        spread: Some(DUMMY_SP),
                        expr: expr.clone(),
                    })
                }
                JSXElementChild::JSXElement(jsx_element) => Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_jsx_element(jsx_element)),
                }),
                JSXElementChild::JSXFragment(jsx_fragment) => Some(ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.transform_jsx_fragment(jsx_fragment)),
                }),
            })
            .map(Some)
            .collect::<Vec<_>>();

        let slot_flag = self.slot_flag_stack.pop().unwrap_or(SlotFlag::Stable);

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
                            alt: Box::new(self.wrap_children(elems, slot_flag)),
                        })
                    } else {
                        self.wrap_children(elems, slot_flag)
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
                            alt: Box::new(self.wrap_children(
                                vec![Some(ExprOrSpread {
                                    spread: None,
                                    expr: Box::new(Expr::Ident(slot_ident)),
                                })],
                                slot_flag,
                            )),
                        })
                    } else {
                        self.wrap_children(elems, slot_flag)
                    }
                }
                expr @ Expr::Fn(..) | expr @ Expr::Arrow(..) => Expr::Object(ObjectLit {
                    span: DUMMY_SP,
                    props: vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: PropName::Ident(quote_ident!("default")),
                        value: Box::new(expr.clone()),
                    })))],
                }),
                Expr::Object(ObjectLit { props, .. }) => {
                    let mut props = props.clone();
                    if self.options.optimize {
                        props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(quote_ident!("_")),
                            value: Box::new(Expr::Lit(Lit::Num(Number {
                                span: DUMMY_SP,
                                value: slot_flag as u8 as f64,
                                raw: None,
                            }))),
                        }))));
                    }
                    Expr::Object(ObjectLit {
                        span: DUMMY_SP,
                        props,
                    })
                }
                _ => {
                    if is_component {
                        self.wrap_children(elems, slot_flag)
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
                    self.wrap_children(elems, slot_flag)
                } else {
                    Expr::Array(ArrayLit {
                        span: DUMMY_SP,
                        elems,
                    })
                }
            }
        }
    }

    fn wrap_children(&mut self, elems: Vec<Option<ExprOrSpread>>, slot_flag: SlotFlag) -> Expr {
        let mut props = vec![PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
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
        })))];

        if self.options.optimize {
            props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(quote_ident!("_")),
                value: Box::new(Expr::Lit(Lit::Num(Number {
                    span: DUMMY_SP,
                    value: slot_flag as u8 as f64,
                    raw: None,
                }))),
            }))));
        }

        Expr::Object(ObjectLit {
            span: DUMMY_SP,
            props,
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

    fn transform_jsx_text(&mut self, jsx_text: &JSXText) -> Option<Expr> {
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
        if text.is_empty() {
            None
        } else {
            Some(Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(Expr::Ident(
                    self.import_from_vue("createTextVNode"),
                ))),
                args: vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(quote_str!(text)))),
                }],
                type_args: None,
            }))
        }
    }

    fn resolve_directive(&mut self, directive_name: &str, jsx_element: &JSXElement) -> Expr {
        match directive_name {
            "show" => Expr::Ident(self.import_from_vue("vShow")),
            "model" => match &jsx_element.opening.name {
                JSXElementName::Ident(ident) if &ident.sym == "select" => {
                    Expr::Ident(self.import_from_vue("vModelSelect"))
                }
                JSXElementName::Ident(ident) if &ident.sym == "textarea" => {
                    Expr::Ident(self.import_from_vue("vModelText"))
                }
                _ => {
                    let typ = jsx_element
                        .opening
                        .attrs
                        .iter()
                        .find_map(|jsx_attr_or_spread| match jsx_attr_or_spread {
                            JSXAttrOrSpread::JSXAttr(JSXAttr {
                                name: JSXAttrName::Ident(ident),
                                value,
                                ..
                            }) if &ident.sym == "type" => value.as_ref(),
                            _ => None,
                        });
                    match typ {
                        Some(JSXAttrValue::Lit(Lit::Str(str))) if &str.value == "checkbox" => {
                            Expr::Ident(self.import_from_vue("vModelCheckbox"))
                        }
                        Some(JSXAttrValue::Lit(Lit::Str(str))) if &str.value == "radio" => {
                            Expr::Ident(self.import_from_vue("vModelRadio"))
                        }
                        Some(JSXAttrValue::Lit(Lit::Str(..))) | None => {
                            Expr::Ident(self.import_from_vue("vModelText"))
                        }
                        Some(..) => Expr::Ident(self.import_from_vue("vModelDynamic")),
                    }
                }
            },
            _ => Expr::Call(CallExpr {
                span: DUMMY_SP,
                callee: Callee::Expr(Box::new(Expr::Ident(
                    self.import_from_vue("resolveDirective"),
                ))),
                args: vec![ExprOrSpread {
                    spread: None,
                    expr: Box::new(Expr::Lit(Lit::Str(quote_str!(directive_name)))),
                }],
                type_args: None,
            }),
        }
    }

    fn is_component(&self, element_name: &JSXElementName) -> bool {
        let name = match element_name {
            JSXElementName::Ident(Ident { sym, .. }) => sym,
            JSXElementName::JSXMemberExpr(JSXMemberExpr { prop, .. }) => &*prop.sym,
            JSXElementName::JSXNamespacedName(JSXNamespacedName { name, .. }) => &*name.sym,
        };
        let should_transformed_to_slots = !self
            .vue_imports
            .get(FRAGMENT)
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

    fn get_pragma(&mut self) -> Ident {
        self.pragma
            .as_ref()
            .or_else(|| self.options.pragma.as_ref())
            .map(|name| quote_ident!(name.as_str()))
            .unwrap_or_else(|| self.import_from_vue("createVNode"))
    }

    fn search_jsx_pragma(&mut self, span: Span) {
        if let Some(comments) = &self.comments {
            comments.with_leading(span.lo, |comments| {
                let pragma = comments.iter().find_map(|comment| {
                    let trimmed = comment.text.trim();
                    trimmed
                        .strip_prefix('*')
                        .unwrap_or(trimmed)
                        .trim()
                        .strip_prefix("@jsx")
                        .map(str::trim)
                });
                if let Some(pragma) = pragma {
                    self.pragma = Some(pragma.to_string());
                }
            });
        }
    }
}

impl<C> VisitMut for VueJsxTransformVisitor<C>
where
    C: Comments,
{
    fn visit_mut_module(&mut self, module: &mut Module) {
        self.search_jsx_pragma(module.span);
        module
            .body
            .iter()
            .for_each(|item| self.search_jsx_pragma(item.span()));

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

        if let Some(helper) = &self.transform_on_helper {
            module.body.insert(
                0,
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                    span: DUMMY_SP,
                    specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                        span: DUMMY_SP,
                        local: helper.clone(),
                    })],
                    src: Box::new(quote_str!("@vue/babel-helper-vue-transform-on")),
                    type_only: false,
                    asserts: None,
                })),
            )
        }

        if !self.vue_imports.is_empty() {
            module.body.insert(
                0,
                ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                    span: DUMMY_SP,
                    specifiers: self
                        .vue_imports
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
    program.fold_with(&mut as_folder(VueJsxTransformVisitor::new(
        options,
        metadata.unresolved_mark,
        metadata.comments,
    )))
}
