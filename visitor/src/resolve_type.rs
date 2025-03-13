use crate::VueJsxTransformVisitor;
use indexmap::{IndexMap, IndexSet};
use std::borrow::Cow;
use swc_core::{
    common::{comments::Comments, EqIgnoreSpan, Span, Spanned, DUMMY_SP},
    ecma::{
        ast::*,
        atoms::{js_word, JsWord},
        utils::{quote_ident, quote_str},
    },
    plugin::errors::HANDLER,
};

enum RefinedTsTypeElement {
    Property(TsPropertySignature),
    GetterSignature(TsGetterSignature),
    MethodSignature(TsMethodSignature),
    CallSignature(TsCallSignatureDecl),
}

struct PropIr {
    types: IndexSet<Option<JsWord>>,
    required: bool,
}

impl<C> VueJsxTransformVisitor<C>
where
    C: Comments,
{
    pub(crate) fn extract_props_type(&mut self, setup_fn: &ExprOrSpread) -> Option<Expr> {
        let mut defaults = None;
        let first_param_type = if let ExprOrSpread { expr, spread: None } = setup_fn {
            match &**expr {
                Expr::Arrow(arrow) => arrow.params.first().and_then(|param| {
                    if let Pat::Assign(AssignPat { right, .. }) = param {
                        defaults = Some(&**right);
                    }
                    extract_type_ann_from_pat(param)
                }),
                Expr::Fn(fn_expr) => fn_expr.function.params.first().and_then(|param| {
                    if let Pat::Assign(AssignPat { right, .. }) = &param.pat {
                        defaults = Some(&**right);
                    }
                    extract_type_ann_from_pat(&param.pat)
                }),
                _ => None,
            }?
        } else {
            return None;
        };

        enum Defaults<'n> {
            Static(Vec<(Cow<'n, PropName>, Expr)>),
            Dynamic(&'n Expr),
        }
        let defaults = defaults.map(|defaults| {
            if let Expr::Object(ObjectLit { props, .. }) = defaults {
                if let Some(props) = props
                    .iter()
                    .map(|prop| {
                        if let PropOrSpread::Prop(prop) = prop {
                            match &**prop {
                                Prop::Shorthand(ident) => Some((
                                    Cow::Owned(PropName::Ident(ident.clone().into())),
                                    Expr::Arrow(ArrowExpr {
                                        params: vec![],
                                        body: Box::new(BlockStmtOrExpr::Expr(Box::new(
                                            Expr::Ident(ident.clone()),
                                        ))),
                                        is_async: false,
                                        is_generator: false,
                                        span: DUMMY_SP,
                                        ..Default::default()
                                    }),
                                )),
                                Prop::KeyValue(KeyValueProp { key, value }) => {
                                    try_unwrap_lit_prop_name(key).map(|key| {
                                        (
                                            key,
                                            if value.is_lit() {
                                                (**value).clone()
                                            } else {
                                                Expr::Arrow(ArrowExpr {
                                                    params: vec![],
                                                    body: Box::new(BlockStmtOrExpr::Expr(
                                                        value.clone(),
                                                    )),
                                                    is_async: false,
                                                    is_generator: false,
                                                    span: DUMMY_SP,
                                                    ..Default::default()
                                                })
                                            },
                                        )
                                    })
                                }
                                Prop::Getter(GetterProp {
                                    key,
                                    body: Some(body),
                                    ..
                                }) => try_unwrap_lit_prop_name(key).map(|key| {
                                    (
                                        key,
                                        Expr::Arrow(ArrowExpr {
                                            params: vec![],
                                            body: Box::new(BlockStmtOrExpr::BlockStmt(
                                                body.clone(),
                                            )),
                                            is_async: false,
                                            is_generator: false,
                                            span: DUMMY_SP,
                                            ..Default::default()
                                        }),
                                    )
                                }),
                                Prop::Method(MethodProp { key, function }) => {
                                    try_unwrap_lit_prop_name(key).map(|key| {
                                        (
                                            key,
                                            Expr::Fn(FnExpr {
                                                ident: None,
                                                function: function.clone(),
                                            }),
                                        )
                                    })
                                }
                                _ => None,
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Option<Vec<_>>>()
                {
                    Defaults::Static(props)
                } else {
                    Defaults::Dynamic(defaults)
                }
            } else {
                Defaults::Dynamic(defaults)
            }
        });

        Some(match defaults {
            Some(Defaults::Static(props)) => {
                Expr::Object(self.build_props_type(first_param_type, Some(props)))
            }
            Some(Defaults::Dynamic(expr)) => {
                let merge_defaults = self.import_from_vue("mergeDefaults");
                Expr::Call(CallExpr {
                    callee: Callee::Expr(Box::new(Expr::Ident(merge_defaults))),
                    args: vec![
                        ExprOrSpread {
                            expr: Box::new(Expr::Object(
                                self.build_props_type(first_param_type, None),
                            )),
                            spread: None,
                        },
                        ExprOrSpread {
                            expr: Box::new(expr.clone()),
                            spread: None,
                        },
                    ],
                    span: if let Some(comments) = &self.comments {
                        let span = Span::dummy_with_cmt();
                        comments.add_pure_comment(span.lo);
                        span
                    } else {
                        DUMMY_SP
                    },
                    ..Default::default()
                })
            }
            None => Expr::Object(self.build_props_type(first_param_type, None)),
        })
    }

    fn build_props_type(
        &self,
        TsTypeAnn { type_ann, .. }: &TsTypeAnn,
        defaults: Option<Vec<(Cow<PropName>, Expr)>>,
    ) -> ObjectLit {
        let mut props = Vec::with_capacity(3);
        self.resolve_type_elements(type_ann, &mut props);

        let cap = props.len();
        let irs = props.into_iter().fold(
            IndexMap::<PropName, PropIr>::with_capacity(cap),
            |mut irs, prop| {
                match prop {
                    RefinedTsTypeElement::Property(TsPropertySignature {
                        key,
                        computed,
                        optional,
                        type_ann,
                        ..
                    }) => {
                        let prop_name = extract_prop_name(*key, computed);
                        let types = if let Some(type_ann) = type_ann {
                            self.infer_runtime_type(&type_ann.type_ann)
                        } else {
                            let mut types = IndexSet::with_capacity(1);
                            types.insert(None);
                            types
                        };
                        if let Some((_, ir)) = irs
                            .iter_mut()
                            .find(|(key, _)| prop_name.eq_ignore_span(key))
                        {
                            if optional {
                                ir.required = false;
                            }
                            ir.types.extend(types);
                        } else {
                            irs.insert(
                                prop_name,
                                PropIr {
                                    types,
                                    required: !optional,
                                },
                            );
                        }
                    }
                    RefinedTsTypeElement::GetterSignature(TsGetterSignature {
                        key,
                        computed,
                        type_ann,
                        ..
                    }) => {
                        let prop_name = extract_prop_name(*key, computed);
                        let types = if let Some(type_ann) = type_ann {
                            self.infer_runtime_type(&type_ann.type_ann)
                        } else {
                            let mut types = IndexSet::with_capacity(1);
                            types.insert(None);
                            types
                        };
                        if let Some((_, ir)) = irs
                            .iter_mut()
                            .find(|(key, _)| prop_name.eq_ignore_span(key))
                        {
                            ir.types.extend(types);
                        } else {
                            irs.insert(
                                prop_name,
                                PropIr {
                                    types,
                                    required: true,
                                },
                            );
                        }
                    }
                    RefinedTsTypeElement::MethodSignature(TsMethodSignature {
                        key,
                        computed,
                        optional,
                        ..
                    }) => {
                        let prop_name = extract_prop_name(*key, computed);
                        let ty = Some(js_word!("Function"));
                        if let Some((_, ir)) = irs
                            .iter_mut()
                            .find(|(key, _)| prop_name.eq_ignore_span(key))
                        {
                            if optional {
                                ir.required = false;
                            }
                            ir.types.insert(ty);
                        } else {
                            let mut types = IndexSet::with_capacity(1);
                            types.insert(ty);
                            irs.insert(
                                prop_name,
                                PropIr {
                                    types,
                                    required: !optional,
                                },
                            );
                        }
                    }
                    RefinedTsTypeElement::CallSignature(..) => {}
                }
                irs
            },
        );

        ObjectLit {
            props: irs
                .into_iter()
                .map(|(prop_name, mut ir)| {
                    let mut props = vec![
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(quote_ident!("type")),
                            value: Box::new(if ir.types.len() == 1 {
                                if let Some(ty) = ir.types.pop().unwrap() {
                                    Expr::Ident(quote_ident!(ty).into())
                                } else {
                                    Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
                                }
                            } else {
                                Expr::Array(ArrayLit {
                                    elems: ir
                                        .types
                                        .into_iter()
                                        .map(|ty| {
                                            Some(ExprOrSpread {
                                                expr: Box::new(if let Some(ty) = ty {
                                                    Expr::Ident(quote_ident!(ty).into())
                                                } else {
                                                    Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))
                                                }),
                                                spread: None,
                                            })
                                        })
                                        .collect(),
                                    span: DUMMY_SP,
                                })
                            }),
                        }))),
                        PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(quote_ident!("required")),
                            value: Box::new(Expr::Lit(Lit::Bool(Bool {
                                value: ir.required,
                                span: DUMMY_SP,
                            }))),
                        }))),
                    ];
                    if let Some((_, default)) = defaults.iter().flatten().find(|(name, _)| {
                        name.eq_ignore_span(&prop_name)
                            || if let (
                                PropName::Ident(IdentName { sym: a, .. }),
                                PropName::Str(Str { value: b, .. }),
                            )
                            | (
                                PropName::Str(Str { value: a, .. }),
                                PropName::Ident(IdentName { sym: b, .. }),
                            ) = (&**name, &prop_name)
                            {
                                a == b
                            } else {
                                false
                            }
                    }) {
                        props.push(PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                            key: PropName::Ident(quote_ident!("default")),
                            value: Box::new(default.clone()),
                        }))));
                    }
                    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
                        key: prop_name,
                        value: Box::new(Expr::Object(ObjectLit {
                            props,
                            span: DUMMY_SP,
                        })),
                    })))
                })
                .collect(),
            span: DUMMY_SP,
        }
    }

    fn resolve_type_elements(&self, ty: &TsType, props: &mut Vec<RefinedTsTypeElement>) {
        match ty {
            TsType::TsTypeLit(TsTypeLit { members, .. }) => {
                props.extend(members.iter().filter_map(|member| match member {
                    TsTypeElement::TsPropertySignature(prop) => {
                        Some(RefinedTsTypeElement::Property(prop.clone()))
                    }
                    TsTypeElement::TsMethodSignature(method) => {
                        Some(RefinedTsTypeElement::MethodSignature(method.clone()))
                    }
                    TsTypeElement::TsGetterSignature(getter) => {
                        Some(RefinedTsTypeElement::GetterSignature(getter.clone()))
                    }
                    TsTypeElement::TsCallSignatureDecl(call) => {
                        Some(RefinedTsTypeElement::CallSignature(call.clone()))
                    }
                    _ => None,
                }));
            }
            TsType::TsUnionOrIntersectionType(
                TsUnionOrIntersectionType::TsIntersectionType(TsIntersectionType { types, .. })
                | TsUnionOrIntersectionType::TsUnionType(TsUnionType { types, .. }),
            ) => {
                types
                    .iter()
                    .for_each(|ty| self.resolve_type_elements(ty, props));
            }
            TsType::TsTypeRef(TsTypeRef {
                type_name: TsEntityName::Ident(ident),
                type_params,
                span,
                ..
            }) => {
                let key = (ident.sym.clone(), ident.ctxt);
                if let Some(aliased) = self.type_aliases.get(&key) {
                    self.resolve_type_elements(aliased, props);
                } else if let Some(TsInterfaceDecl {
                    extends,
                    body: TsInterfaceBody { body, .. },
                    ..
                }) = self.interfaces.get(&key)
                {
                    props.extend(body.iter().filter_map(|element| match element {
                        TsTypeElement::TsPropertySignature(prop) => {
                            Some(RefinedTsTypeElement::Property(prop.clone()))
                        }
                        TsTypeElement::TsMethodSignature(method) => {
                            Some(RefinedTsTypeElement::MethodSignature(method.clone()))
                        }
                        TsTypeElement::TsGetterSignature(getter) => {
                            Some(RefinedTsTypeElement::GetterSignature(getter.clone()))
                        }
                        TsTypeElement::TsCallSignatureDecl(call) => {
                            Some(RefinedTsTypeElement::CallSignature(call.clone()))
                        }
                        _ => None,
                    }));
                    extends
                        .iter()
                        .filter_map(|parent| parent.expr.as_ident())
                        .for_each(|ident| {
                            self.resolve_type_elements(
                                &TsType::TsTypeRef(TsTypeRef {
                                    type_name: TsEntityName::Ident(ident.clone()),
                                    type_params: None,
                                    span: DUMMY_SP,
                                }),
                                props,
                            )
                        });
                } else if ident.ctxt.has_mark(self.unresolved_mark) {
                    match &*ident.sym {
                        "Partial" => {
                            if let Some(param) = type_params
                                .as_deref()
                                .and_then(|params| params.params.first())
                            {
                                let mut inner_props = vec![];
                                self.resolve_type_elements(param, &mut inner_props);
                                props.extend(inner_props.into_iter().map(|mut prop| {
                                    match &mut prop {
                                        RefinedTsTypeElement::Property(property) => {
                                            property.optional = true;
                                        }
                                        RefinedTsTypeElement::MethodSignature(method) => {
                                            method.optional = true;
                                        }
                                        RefinedTsTypeElement::GetterSignature(..) => {}
                                        RefinedTsTypeElement::CallSignature(..) => {}
                                    }
                                    prop
                                }));
                            }
                        }
                        "Required" => {
                            if let Some(param) = type_params
                                .as_deref()
                                .and_then(|params| params.params.first())
                            {
                                let mut inner_props = vec![];
                                self.resolve_type_elements(param, &mut inner_props);
                                props.extend(inner_props.into_iter().map(|mut prop| {
                                    match &mut prop {
                                        RefinedTsTypeElement::Property(TsPropertySignature {
                                            optional,
                                            ..
                                        })
                                        | RefinedTsTypeElement::MethodSignature(
                                            TsMethodSignature { optional, .. },
                                        ) => {
                                            *optional = false;
                                        }
                                        RefinedTsTypeElement::GetterSignature(..)
                                        | RefinedTsTypeElement::CallSignature(..) => {}
                                    }
                                    prop
                                }));
                            }
                        }
                        "Pick" => {
                            if let Some((object, keys)) = type_params
                                .as_deref()
                                .and_then(|params| params.params.first().zip(params.params.get(1)))
                            {
                                let keys = self.resolve_string_or_union_strings(keys);
                                let mut inner_props = vec![];
                                self.resolve_type_elements(object, &mut inner_props);
                                props.extend(inner_props.into_iter().filter(|prop| match prop {
                                    RefinedTsTypeElement::Property(TsPropertySignature {
                                        key,
                                        ..
                                    })
                                    | RefinedTsTypeElement::MethodSignature(TsMethodSignature {
                                        key,
                                        ..
                                    })
                                    | RefinedTsTypeElement::GetterSignature(TsGetterSignature {
                                        key,
                                        ..
                                    }) => match &**key {
                                        Expr::Ident(ident) => keys.contains(&ident.sym),
                                        Expr::Lit(Lit::Str(str)) => keys.contains(&str.value),
                                        _ => false,
                                    },
                                    RefinedTsTypeElement::CallSignature(..) => false,
                                }));
                            }
                        }
                        "Omit" => {
                            if let Some((object, keys)) = type_params
                                .as_deref()
                                .and_then(|params| params.params.first().zip(params.params.get(1)))
                            {
                                let keys = self.resolve_string_or_union_strings(keys);
                                let mut inner_props = vec![];
                                self.resolve_type_elements(object, &mut inner_props);
                                props.extend(inner_props.into_iter().filter(|prop| match prop {
                                    RefinedTsTypeElement::Property(TsPropertySignature {
                                        key,
                                        ..
                                    })
                                    | RefinedTsTypeElement::MethodSignature(TsMethodSignature {
                                        key,
                                        ..
                                    })
                                    | RefinedTsTypeElement::GetterSignature(TsGetterSignature {
                                        key,
                                        ..
                                    }) => match &**key {
                                        Expr::Ident(ident) => !keys.contains(&ident.sym),
                                        Expr::Lit(Lit::Str(str)) => !keys.contains(&str.value),
                                        _ => true,
                                    },
                                    RefinedTsTypeElement::CallSignature(..) => true,
                                }));
                            }
                        }
                        _ => {
                            HANDLER.with(|handler| {
                                handler.span_err(
                                    *span,
                                    "Unresolvable type reference or unsupported built-in utility type.",
                                );
                            });
                        }
                    }
                } else {
                    HANDLER.with(|handler| {
                        handler.span_err(*span, "Types from other modules can't be resolved.");
                    });
                }
            }
            TsType::TsIndexedAccessType(TsIndexedAccessType {
                obj_type,
                index_type,
                ..
            }) => {
                if let Some(ty) = self.resolve_indexed_access(obj_type, index_type) {
                    self.resolve_type_elements(&ty, props);
                } else {
                    HANDLER.with(|handler| {
                        handler.span_err(ty.span(), "Unresolvable type.");
                    });
                }
            }
            TsType::TsFnOrConstructorType(TsFnOrConstructorType::TsFnType(TsFnType {
                params,
                type_params,
                type_ann,
                ..
            })) => {
                props.push(RefinedTsTypeElement::CallSignature(TsCallSignatureDecl {
                    params: params.clone(),
                    type_ann: Some(type_ann.clone()),
                    type_params: type_params.clone(),
                    span: DUMMY_SP,
                }));
            }
            TsType::TsParenthesizedType(TsParenthesizedType { type_ann, .. })
            | TsType::TsOptionalType(TsOptionalType { type_ann, .. }) => {
                self.resolve_type_elements(type_ann, props);
            }
            _ => HANDLER.with(|handler| {
                handler.span_err(ty.span(), "Unresolvable type.");
            }),
        }
    }

    fn resolve_string_or_union_strings(&self, ty: &TsType) -> Vec<JsWord> {
        match ty {
            TsType::TsLitType(TsLitType {
                lit: TsLit::Str(key),
                ..
            }) => vec![key.value.clone()],
            TsType::TsUnionOrIntersectionType(TsUnionOrIntersectionType::TsUnionType(
                TsUnionType { types, .. },
            )) => types
                .iter()
                .fold(Vec::with_capacity(types.len()), |mut strings, ty| {
                    if let TsType::TsLitType(TsLitType {
                        lit: TsLit::Str(str),
                        ..
                    }) = &**ty
                    {
                        strings.push(str.value.clone());
                    } else {
                        strings.extend_from_slice(&self.resolve_string_or_union_strings(ty));
                    }
                    strings
                }),
            TsType::TsTypeRef(TsTypeRef {
                type_name: TsEntityName::Ident(ident),
                ..
            }) => {
                if let Some(aliased) = self.type_aliases.get(&(ident.sym.clone(), ident.ctxt)) {
                    self.resolve_string_or_union_strings(aliased)
                } else if ident.ctxt.has_mark(self.unresolved_mark) {
                    HANDLER.with(|handler| {
                        handler.span_err(
                            ty.span(),
                            "Unresolvable type reference or unsupported built-in utility type.",
                        );
                    });
                    vec![]
                } else {
                    HANDLER.with(|handler| {
                        handler.span_err(ty.span(), "Types from other modules can't be resolved.");
                    });
                    vec![]
                }
            }
            _ => {
                HANDLER
                    .with(|handler| handler.span_err(ty.span(), "Unsupported type as index key."));
                vec![]
            }
        }
    }

    fn resolve_indexed_access(&self, obj: &TsType, index: &TsType) -> Option<TsType> {
        match obj {
            TsType::TsTypeRef(TsTypeRef {
                type_name: TsEntityName::Ident(ident),
                type_params,
                ..
            }) => {
                let key = (ident.sym.clone(), ident.ctxt);
                if let Some(aliased) = self.type_aliases.get(&key) {
                    self.resolve_indexed_access(aliased, index)
                } else if let Some(interface) = self.interfaces.get(&key) {
                    let mut properties = match index {
                        TsType::TsKeywordType(TsKeywordType {
                            kind: TsKeywordTypeKind::TsStringKeyword,
                            ..
                        }) => interface
                            .body
                            .body
                            .iter()
                            .filter_map(|element| match element {
                                TsTypeElement::TsCallSignatureDecl(..)
                                | TsTypeElement::TsConstructSignatureDecl(..)
                                | TsTypeElement::TsSetterSignature(..) => None,
                                TsTypeElement::TsPropertySignature(TsPropertySignature {
                                    key,
                                    type_ann,
                                    ..
                                })
                                | TsTypeElement::TsGetterSignature(TsGetterSignature {
                                    key,
                                    type_ann,
                                    ..
                                }) => {
                                    if matches!(&**key, Expr::Ident(..) | Expr::Lit(Lit::Str(..))) {
                                        type_ann.as_ref().map(|type_ann| type_ann.type_ann.clone())
                                    } else {
                                        None
                                    }
                                }
                                TsTypeElement::TsIndexSignature(TsIndexSignature {
                                    type_ann,
                                    ..
                                }) => type_ann.as_ref().map(|type_ann| type_ann.type_ann.clone()),
                                TsTypeElement::TsMethodSignature(..) => {
                                    Some(Box::new(TsType::TsTypeRef(TsTypeRef {
                                        type_name: TsEntityName::Ident(
                                            quote_ident!("Function").into(),
                                        ),
                                        type_params: None,
                                        span: DUMMY_SP,
                                    })))
                                }
                            })
                            .collect(),
                        TsType::TsLitType(TsLitType {
                            lit: TsLit::Str(..),
                            ..
                        })
                        | TsType::TsUnionOrIntersectionType(
                            TsUnionOrIntersectionType::TsUnionType(..),
                        )
                        | TsType::TsTypeRef(..) => {
                            let keys = self.resolve_string_or_union_strings(index);
                            interface
                                .body
                                .body
                                .iter()
                                .filter_map(|element| match element {
                                    TsTypeElement::TsPropertySignature(TsPropertySignature {
                                        key,
                                        type_ann,
                                        ..
                                    })
                                    | TsTypeElement::TsGetterSignature(TsGetterSignature {
                                        key,
                                        type_ann,
                                        ..
                                    }) => {
                                        if let Expr::Ident(Ident { sym: key, .. })
                                        | Expr::Lit(Lit::Str(Str { value: key, .. })) = &**key
                                        {
                                            if keys.contains(key) {
                                                type_ann
                                                    .as_ref()
                                                    .map(|type_ann| type_ann.type_ann.clone())
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    }
                                    TsTypeElement::TsMethodSignature(TsMethodSignature {
                                        key,
                                        ..
                                    }) => {
                                        if let Expr::Ident(Ident { sym: key, .. })
                                        | Expr::Lit(Lit::Str(Str { value: key, .. })) = &**key
                                        {
                                            if keys.contains(key) {
                                                Some(Box::new(TsType::TsTypeRef(TsTypeRef {
                                                    type_name: TsEntityName::Ident(
                                                        quote_ident!("Function").into(),
                                                    ),
                                                    type_params: None,
                                                    span: DUMMY_SP,
                                                })))
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    }
                                    TsTypeElement::TsCallSignatureDecl(..)
                                    | TsTypeElement::TsConstructSignatureDecl(..)
                                    | TsTypeElement::TsSetterSignature(..)
                                    | TsTypeElement::TsIndexSignature(..) => None,
                                })
                                .collect()
                        }
                        _ => vec![],
                    };
                    if properties.len() == 1 {
                        Some((*properties.remove(0)).clone())
                    } else {
                        Some(TsType::TsUnionOrIntersectionType(
                            TsUnionOrIntersectionType::TsUnionType(TsUnionType {
                                types: properties,
                                span: DUMMY_SP,
                            }),
                        ))
                    }
                } else if ident.ctxt.has_mark(self.unresolved_mark) {
                    if ident.sym == "Array" {
                        type_params
                            .as_ref()
                            .and_then(|params| params.params.first())
                            .map(|ty| (**ty).clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            TsType::TsTypeLit(TsTypeLit { members, .. }) => {
                let mut properties = match index {
                    TsType::TsKeywordType(TsKeywordType {
                        kind: TsKeywordTypeKind::TsStringKeyword,
                        ..
                    }) => members
                        .iter()
                        .filter_map(|member| match member {
                            TsTypeElement::TsCallSignatureDecl(..)
                            | TsTypeElement::TsConstructSignatureDecl(..)
                            | TsTypeElement::TsSetterSignature(..) => None,
                            TsTypeElement::TsPropertySignature(TsPropertySignature {
                                key,
                                type_ann,
                                ..
                            })
                            | TsTypeElement::TsGetterSignature(TsGetterSignature {
                                key,
                                type_ann,
                                ..
                            }) => {
                                if matches!(&**key, Expr::Ident(..) | Expr::Lit(Lit::Str(..))) {
                                    type_ann.as_ref().map(|type_ann| type_ann.type_ann.clone())
                                } else {
                                    None
                                }
                            }
                            TsTypeElement::TsIndexSignature(TsIndexSignature {
                                type_ann, ..
                            }) => type_ann.as_ref().map(|type_ann| type_ann.type_ann.clone()),
                            TsTypeElement::TsMethodSignature(..) => {
                                Some(Box::new(TsType::TsTypeRef(TsTypeRef {
                                    type_name: TsEntityName::Ident(quote_ident!("Function").into()),
                                    type_params: None,
                                    span: DUMMY_SP,
                                })))
                            }
                        })
                        .collect(),
                    TsType::TsLitType(TsLitType {
                        lit: TsLit::Str(..),
                        ..
                    })
                    | TsType::TsTypeRef(..)
                    | TsType::TsUnionOrIntersectionType(TsUnionOrIntersectionType::TsUnionType(
                        ..,
                    )) => {
                        let keys = self.resolve_string_or_union_strings(index);
                        members
                            .iter()
                            .filter_map(|member| match member {
                                TsTypeElement::TsPropertySignature(TsPropertySignature {
                                    key,
                                    type_ann,
                                    ..
                                })
                                | TsTypeElement::TsGetterSignature(TsGetterSignature {
                                    key,
                                    type_ann,
                                    ..
                                }) => {
                                    if let Expr::Ident(Ident { sym: key, .. })
                                    | Expr::Lit(Lit::Str(Str { value: key, .. })) = &**key
                                    {
                                        if keys.contains(key) {
                                            type_ann
                                                .as_ref()
                                                .map(|type_ann| type_ann.type_ann.clone())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                }
                                TsTypeElement::TsMethodSignature(TsMethodSignature {
                                    key, ..
                                }) => {
                                    if let Expr::Ident(Ident { sym: key, .. })
                                    | Expr::Lit(Lit::Str(Str { value: key, .. })) = &**key
                                    {
                                        if keys.contains(key) {
                                            Some(Box::new(TsType::TsTypeRef(TsTypeRef {
                                                type_name: TsEntityName::Ident(
                                                    quote_ident!("Function").into(),
                                                ),
                                                type_params: None,
                                                span: DUMMY_SP,
                                            })))
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                }
                                TsTypeElement::TsCallSignatureDecl(..)
                                | TsTypeElement::TsConstructSignatureDecl(..)
                                | TsTypeElement::TsSetterSignature(..)
                                | TsTypeElement::TsIndexSignature(..) => None,
                            })
                            .collect()
                    }
                    _ => vec![],
                };
                if properties.len() == 1 {
                    Some(*properties.remove(0))
                } else {
                    Some(TsType::TsUnionOrIntersectionType(
                        TsUnionOrIntersectionType::TsUnionType(TsUnionType {
                            types: properties,
                            span: DUMMY_SP,
                        }),
                    ))
                }
            }
            TsType::TsArrayType(TsArrayType { elem_type, .. }) => {
                if matches!(
                    index,
                    TsType::TsKeywordType(TsKeywordType {
                        kind: TsKeywordTypeKind::TsNumberKeyword,
                        ..
                    }) | TsType::TsLitType(TsLitType {
                        lit: TsLit::Number(..),
                        ..
                    })
                ) {
                    Some((**elem_type).clone())
                } else {
                    None
                }
            }
            TsType::TsTupleType(TsTupleType { elem_types, .. }) => match index {
                TsType::TsLitType(TsLitType {
                    lit: TsLit::Number(num),
                    ..
                }) => elem_types
                    .get(num.value as usize)
                    .map(|element| (*element.ty).clone()),
                TsType::TsKeywordType(TsKeywordType {
                    kind: TsKeywordTypeKind::TsNumberKeyword,
                    ..
                }) => Some(TsType::TsUnionOrIntersectionType(
                    TsUnionOrIntersectionType::TsUnionType(TsUnionType {
                        types: elem_types
                            .iter()
                            .map(|TsTupleElement { ty, .. }| ty.clone())
                            .collect(),
                        span: DUMMY_SP,
                    }),
                )),
                _ => None,
            },
            _ => None,
        }
    }

    fn infer_runtime_type(&self, ty: &TsType) -> IndexSet<Option<JsWord>> {
        let mut runtime_types = IndexSet::with_capacity(1);
        match ty {
            TsType::TsKeywordType(keyword) => match keyword.kind {
                TsKeywordTypeKind::TsStringKeyword => {
                    runtime_types.insert(Some(js_word!("String")));
                }
                TsKeywordTypeKind::TsNumberKeyword => {
                    runtime_types.insert(Some(js_word!("Number")));
                }
                TsKeywordTypeKind::TsBooleanKeyword => {
                    runtime_types.insert(Some(js_word!("Boolean")));
                }
                TsKeywordTypeKind::TsObjectKeyword => {
                    runtime_types.insert(Some(js_word!("Object")));
                }
                TsKeywordTypeKind::TsNullKeyword => {
                    runtime_types.insert(None);
                }
                TsKeywordTypeKind::TsBigIntKeyword => {
                    runtime_types.insert(Some(js_word!("BigInt")));
                }
                TsKeywordTypeKind::TsSymbolKeyword => {
                    runtime_types.insert(Some(js_word!("Symbol")));
                }
                _ => {
                    runtime_types.insert(None);
                }
            },
            TsType::TsTypeLit(TsTypeLit { members, .. }) => {
                members.iter().for_each(|member| {
                    if let TsTypeElement::TsCallSignatureDecl(..)
                    | TsTypeElement::TsConstructSignatureDecl(..) = member
                    {
                        runtime_types.insert(Some(js_word!("Function")));
                    } else {
                        runtime_types.insert(Some(js_word!("Object")));
                    }
                });
            }
            TsType::TsFnOrConstructorType(..) => {
                runtime_types.insert(Some(js_word!("Function")));
            }
            TsType::TsArrayType(..) | TsType::TsTupleType(..) => {
                runtime_types.insert(Some(js_word!("Array")));
            }
            TsType::TsLitType(TsLitType { lit, .. }) => match lit {
                TsLit::Str(..) | TsLit::Tpl(..) => {
                    runtime_types.insert(Some(js_word!("String")));
                }
                TsLit::Bool(..) => {
                    runtime_types.insert(Some(js_word!("Boolean")));
                }
                TsLit::Number(..) | TsLit::BigInt(..) => {
                    runtime_types.insert(Some(js_word!("Number")));
                }
            },
            TsType::TsTypeRef(TsTypeRef {
                type_name: TsEntityName::Ident(ident),
                type_params,
                ..
            }) => {
                let key = (ident.sym.clone(), ident.ctxt);
                if let Some(aliased) = self.type_aliases.get(&key) {
                    runtime_types.extend(self.infer_runtime_type(aliased));
                } else if let Some(TsInterfaceDecl {
                    body: TsInterfaceBody { body, .. },
                    ..
                }) = self.interfaces.get(&key)
                {
                    body.iter().for_each(|element| {
                        if let TsTypeElement::TsCallSignatureDecl(..)
                        | TsTypeElement::TsConstructSignatureDecl(..) = element
                        {
                            runtime_types.insert(Some(js_word!("Function")));
                        } else {
                            runtime_types.insert(Some(js_word!("Object")));
                        }
                    });
                } else {
                    match &*ident.sym {
                        "Array" | "Function" | "Object" | "Set" | "Map" | "WeakSet" | "WeakMap"
                        | "Date" | "Promise" | "Error" | "RegExp" => {
                            runtime_types.insert(Some(ident.sym.clone()));
                        }
                        "Partial" | "Required" | "Readonly" | "Record" | "Pick" | "Omit"
                        | "InstanceType" => {
                            runtime_types.insert(Some(js_word!("Object")));
                        }
                        "Uppercase" | "Lowercase" | "Capitalize" | "Uncapitalize" => {
                            runtime_types.insert(Some(js_word!("String")));
                        }
                        "Parameters" | "ConstructorParameters" => {
                            runtime_types.insert(Some(js_word!("Array")));
                        }
                        "NonNullable" => {
                            if let Some(ty) = type_params
                                .as_ref()
                                .and_then(|type_params| type_params.params.first())
                            {
                                let types = self.infer_runtime_type(ty);
                                runtime_types.extend(types.into_iter().filter(|ty| ty.is_some()));
                            } else {
                                runtime_types.insert(Some(js_word!("Object")));
                            }
                        }
                        "Exclude" | "OmitThisParameter" => {
                            if let Some(ty) = type_params
                                .as_ref()
                                .and_then(|type_params| type_params.params.first())
                            {
                                runtime_types.extend(self.infer_runtime_type(ty));
                            } else {
                                runtime_types.insert(Some(js_word!("Object")));
                            }
                        }
                        "Extract" => {
                            if let Some(ty) = type_params
                                .as_ref()
                                .and_then(|type_params| type_params.params.get(1))
                            {
                                runtime_types.extend(self.infer_runtime_type(ty));
                            } else {
                                runtime_types.insert(Some(js_word!("Object")));
                            }
                        }
                        _ => {
                            runtime_types.insert(Some(js_word!("Object")));
                        }
                    }
                }
            }
            TsType::TsParenthesizedType(TsParenthesizedType { type_ann, .. }) => {
                runtime_types.extend(self.infer_runtime_type(type_ann));
            }
            TsType::TsUnionOrIntersectionType(
                TsUnionOrIntersectionType::TsUnionType(TsUnionType { types, .. })
                | TsUnionOrIntersectionType::TsIntersectionType(TsIntersectionType { types, .. }),
            ) => runtime_types.extend(types.iter().flat_map(|ty| self.infer_runtime_type(ty))),
            TsType::TsIndexedAccessType(TsIndexedAccessType {
                obj_type,
                index_type,
                ..
            }) => {
                if let Some(ty) = self.resolve_indexed_access(obj_type, index_type) {
                    runtime_types.extend(self.infer_runtime_type(&ty));
                }
            }
            TsType::TsOptionalType(TsOptionalType { type_ann, .. }) => {
                runtime_types.extend(self.infer_runtime_type(type_ann));
            }
            _ => {
                runtime_types.insert(Some(js_word!("Object")));
            }
        };
        runtime_types
    }

    pub(crate) fn extract_emits_type(&self, setup_fn: &ExprOrSpread) -> Option<ArrayLit> {
        let TsTypeAnn {
            type_ann: second_param_type,
            ..
        } = if let ExprOrSpread { expr, spread: None } = setup_fn {
            match &**expr {
                Expr::Arrow(arrow) => match arrow.params.get(1) {
                    Some(Pat::Ident(ident)) => ident.type_ann.as_deref(),
                    Some(Pat::Array(array)) => array.type_ann.as_deref(),
                    Some(Pat::Object(object)) => object.type_ann.as_deref(),
                    _ => return None,
                },
                Expr::Fn(fn_expr) => match fn_expr.function.params.get(1).map(|param| &param.pat) {
                    Some(Pat::Ident(ident)) => ident.type_ann.as_deref(),
                    Some(Pat::Array(array)) => array.type_ann.as_deref(),
                    Some(Pat::Object(object)) => object.type_ann.as_deref(),
                    _ => return None,
                },
                _ => return None,
            }?
        } else {
            return None;
        };

        match &**second_param_type {
            TsType::TsTypeRef(TsTypeRef {
                type_name: TsEntityName::Ident(ident),
                type_params: Some(type_params),
                ..
            }) if ident.sym == "SetupContext" => {
                if let Some(emits_def) = type_params.params.first() {
                    let mut emits = Vec::with_capacity(1);
                    self.resolve_type_elements(emits_def, &mut emits);
                    Some(ArrayLit {
                        elems: emits
                            .into_iter()
                            .flat_map(|emit| match emit {
                                RefinedTsTypeElement::MethodSignature(TsMethodSignature {
                                    key,
                                    ..
                                })
                                | RefinedTsTypeElement::Property(TsPropertySignature {
                                    key, ..
                                }) => match &*key {
                                    Expr::Ident(ident) => vec![ident.sym.clone()],
                                    Expr::Lit(Lit::Str(str)) => vec![str.value.clone()],
                                    _ => vec![],
                                },
                                RefinedTsTypeElement::CallSignature(TsCallSignatureDecl {
                                    params,
                                    ..
                                }) => params
                                    .first()
                                    .and_then(|param| match param {
                                        TsFnParam::Ident(ident) => ident.type_ann.as_deref(),
                                        TsFnParam::Array(array) => array.type_ann.as_deref(),
                                        TsFnParam::Rest(rest) => rest.type_ann.as_deref(),
                                        TsFnParam::Object(object) => object.type_ann.as_deref(),
                                    })
                                    .map(|type_ann| {
                                        self.resolve_string_or_union_strings(&type_ann.type_ann)
                                    })
                                    .unwrap_or_default(),
                                RefinedTsTypeElement::GetterSignature(..) => vec![],
                            })
                            .map(|name| {
                                Some(ExprOrSpread {
                                    expr: Box::new(Expr::Lit(Lit::Str(quote_str!(name)))),
                                    spread: None,
                                })
                            })
                            .collect(),
                        span: DUMMY_SP,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

fn extract_prop_name(expr: Expr, computed: bool) -> PropName {
    match expr {
        Expr::Ident(ident) => PropName::Ident(ident.into()),
        Expr::Lit(Lit::Str(str)) => PropName::Str(str),
        Expr::Lit(Lit::Num(num)) => PropName::Num(num),
        Expr::Lit(Lit::BigInt(bigint)) => PropName::BigInt(bigint),
        _ => {
            if computed {
                PropName::Computed(ComputedPropName {
                    expr: Box::new(expr),
                    span: DUMMY_SP,
                })
            } else {
                HANDLER.with(|handler| handler.span_err(expr.span(), "Unsupported prop key."));
                PropName::Ident(quote_ident!(""))
            }
        }
    }
}

fn try_unwrap_lit_prop_name(prop_name: &PropName) -> Option<Cow<PropName>> {
    match prop_name {
        PropName::Ident(..) | PropName::Str(..) | PropName::Num(..) | PropName::BigInt(..) => {
            Some(Cow::Borrowed(prop_name))
        }
        PropName::Computed(ComputedPropName { expr, .. }) => match &**expr {
            Expr::Ident(ident) => Some(Cow::Owned(PropName::Ident(ident.clone().into()))),
            Expr::Lit(Lit::Str(str)) => Some(Cow::Owned(PropName::Str(str.clone()))),
            Expr::Lit(Lit::Num(num)) => Some(Cow::Owned(PropName::Num(num.clone()))),
            Expr::Lit(Lit::BigInt(bigint)) => Some(Cow::Owned(PropName::BigInt(bigint.clone()))),
            _ => None,
        },
    }
}

fn extract_type_ann_from_pat(pat: &Pat) -> Option<&TsTypeAnn> {
    match pat {
        Pat::Ident(ident) => ident.type_ann.as_deref(),
        Pat::Object(object) => object.type_ann.as_deref(),
        Pat::Array(array) => array.type_ann.as_deref(),
        Pat::Assign(assign) => extract_type_ann_from_pat(&assign.left),
        _ => None,
    }
}
