use swc_core::{
    common::DUMMY_SP,
    ecma::{
        ast::*,
        utils::{private_ident, quote_ident, quote_str},
    },
};

pub(crate) fn build_slot_helper(helper_name: Ident, is_vnode: Ident) -> FnDecl {
    let arg = private_ident!("s");

    FnDecl {
        ident: helper_name,
        declare: false,
        function: Box::new(Function {
            params: vec![Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: Pat::Ident(BindingIdent {
                    id: arg.clone(),
                    type_ann: None,
                }),
            }],
            decorators: vec![],
            span: DUMMY_SP,
            body: Some(BlockStmt {
                span: DUMMY_SP,
                stmts: vec![Stmt::Return(ReturnStmt {
                    span: DUMMY_SP,
                    arg: Some(Box::new(Expr::Bin(BinExpr {
                        span: DUMMY_SP,
                        op: op!("||"),
                        left: Box::new(Expr::Bin(BinExpr {
                            span: DUMMY_SP,
                            op: op!("==="),
                            left: Box::new(Expr::Unary(UnaryExpr {
                                span: DUMMY_SP,
                                op: op!("typeof"),
                                arg: Box::new(Expr::Ident(arg.clone())),
                            })),
                            right: Box::new(Expr::Lit(Lit::Str(quote_str!("function")))),
                        })),
                        right: Box::new(Expr::Bin(BinExpr {
                            span: DUMMY_SP,
                            op: op!("&&"),
                            left: Box::new(Expr::Bin(BinExpr {
                                span: DUMMY_SP,
                                op: op!("==="),
                                left: Box::new(Expr::Call(CallExpr {
                                    span: DUMMY_SP,
                                    callee: Callee::Expr(Box::new(Expr::Member(MemberExpr {
                                        span: DUMMY_SP,
                                        obj: Box::new(Expr::Member(MemberExpr {
                                            span: DUMMY_SP,
                                            obj: Box::new(Expr::Object(ObjectLit {
                                                span: DUMMY_SP,
                                                props: vec![],
                                            })),
                                            prop: MemberProp::Ident(quote_ident!("toString")),
                                        })),
                                        prop: MemberProp::Ident(quote_ident!("call")),
                                    }))),
                                    args: vec![ExprOrSpread {
                                        spread: None,
                                        expr: Box::new(Expr::Ident(arg.clone())),
                                    }],
                                    type_args: None,
                                })),
                                right: Box::new(Expr::Lit(Lit::Str(quote_str!("[object Object]")))),
                            })),
                            right: Box::new(Expr::Unary(UnaryExpr {
                                span: DUMMY_SP,
                                op: op!("!"),
                                arg: Box::new(Expr::Call(CallExpr {
                                    span: DUMMY_SP,
                                    callee: Callee::Expr(Box::new(Expr::Ident(is_vnode))),
                                    args: vec![ExprOrSpread {
                                        spread: None,
                                        expr: Box::new(Expr::Ident(arg)),
                                    }],
                                    type_args: None,
                                })),
                            })),
                        })),
                    }))),
                })],
            }),
            is_generator: false,
            is_async: false,
            type_params: None,
            return_type: None,
        }),
    }
}
