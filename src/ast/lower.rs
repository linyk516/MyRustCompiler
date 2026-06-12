use std::vec;

use crate::{
    ast::{
        node::{AstNode, NodeIdAllocator},
        ty::{
            BinaryOp, Block, BlockKind, ElseBranch, Expr, ExprKind, ExternFnDecl, FnDecl, FnSig,
            Ident, Item, ItemKind, Param, Place, PlaceKind, Program, Stmt,
            StmtKind::{self},
            Ty, TyKind,
        },
    },
    compiler::source::SourceFile,
    lexer::token::Span,
    my_grammar::{
        GrammarContext,
        ProdTag::{self},
    },
    parser::{
        CST, ProductionId,
        cst::{CSTNode, CSTNodeID},
    },
};

pub struct Lowerer<'a> {
    cst: &'a CST,
    source: &'a SourceFile,
    ctx: &'a GrammarContext,
    ids: NodeIdAllocator,
    _errors: Vec<LowerError>,
}

#[derive(Debug)]
pub struct LowerError {
    pub message: String,
    pub span: Span,
}

pub type LowerResult<T> = Result<T, LowerError>;

/// 变量定义
struct VarDecl {
    mutable: bool,
    name: Ident,
    ty: Option<Ty>,
}

impl<'a> Lowerer<'a> {
    pub fn new(cst: &'a CST, source: &'a SourceFile, grammar_ctx: &'a GrammarContext) -> Self {
        Self {
            cst,
            source,
            ctx: grammar_ctx,
            ids: NodeIdAllocator::new(),
            _errors: vec![],
        }
    }

    pub fn lower(mut self) -> Result<Program, Vec<LowerError>> {
        let root = self.cst.root();
        Ok(self.lower_program(root).map_err(|err| vec![err])?)
    }
}

impl<'a> Lowerer<'a> {
    fn lower_program(&mut self, node: CSTNodeID) -> LowerResult<Program> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::ProgramDeclList => {
                let [decl_list] = self.expect_children::<1>(node)?;
                Ok(Program {
                    items: self.lower_decl_list(decl_list)?,
                })
            }
            _ => Err(self.unexpected_tag(node, "ProgramDeclList")),
        }
    }

    fn lower_decl_list(&mut self, node: CSTNodeID) -> LowerResult<Vec<Item>> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::DeclListEmpty => Ok(vec![]),
            ProdTag::DeclListDeclDeclList => {
                let [decl_node, decl_list_node] = self.expect_children::<2>(node)?;
                let decl = self.lower_decl(decl_node)?;
                let mut decl_list = self.lower_decl_list(decl_list_node)?;
                decl_list.insert(0, decl);
                Ok(decl_list)
            }
            _ => Err(self.unexpected_tag(node, "Empty, Decl DeclList")),
        }
    }

    fn lower_decl(&mut self, node: CSTNodeID) -> LowerResult<Item> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::DeclFnDecl => {
                let [fn_decl_node] = self.expect_children::<1>(node)?;
                let item = ItemKind::Fn(self.lower_fn_decl(fn_decl_node)?);
                Ok(self.make_item(item, span))
            }
            ProdTag::DeclExternFnDecl => {
                let [extern_fn_decl_node] = self.expect_children::<1>(node)?;
                let item = ItemKind::ExternFn(self.lower_extern_fn_decl(extern_fn_decl_node)?);
                Ok(self.make_item(item, span))
            }
            _ => Err(self.unexpected_tag(node, "FnDecl, ExternFnDecl")),
        }
    }

    fn lower_fn_decl(&mut self, node: CSTNodeID) -> LowerResult<FnDecl> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::FnDeclSigBlock | ProdTag::FnDeclSigBlockExpr => {
                let [fn_sig_node, block_node] = self.expect_children::<2>(node)?;
                let fn_sig = self.lower_fn_sig(fn_sig_node)?;
                let block = self.lower_block(block_node)?;
                Ok(FnDecl {
                    sig: fn_sig,
                    body: block,
                })
            }
            _ => Err(self.unexpected_tag(node, "Sig Block, Sig BlockExpr")),
        }
    }

    fn lower_extern_fn_decl(&mut self, node: CSTNodeID) -> LowerResult<ExternFnDecl> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::ExternFnDeclNoRet => {
                let [_, _, id_node, _, param_list_node, _, _] = self.expect_children::<7>(node)?;
                let (params, variadic) = self.lower_extern_param_list(param_list_node)?;
                let name = self.lower_ident(id_node)?;

                Ok(ExternFnDecl {
                    sig: FnSig {
                        name,
                        params,
                        ret_ty: None,
                        variadic,
                    },
                })
            }
            ProdTag::ExternFnDeclRetTy => {
                let [_, _, id_node, _, param_list_node, _, _, ty_node, _] =
                    self.expect_children::<9>(node)?;
                let (params, variadic) = self.lower_extern_param_list(param_list_node)?;
                let name = self.lower_ident(id_node)?;
                let ret_ty = self.lower_ty(ty_node)?;

                Ok(ExternFnDecl {
                    sig: FnSig {
                        name,
                        params,
                        ret_ty: Some(ret_ty),
                        variadic,
                    },
                })
            }
            _ => Err(self.unexpected_tag(node, "ExternFnDeclNoRet, ExternFnDeclRetTy")),
        }
    }

    fn lower_fn_sig(&mut self, node: CSTNodeID) -> LowerResult<FnSig> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::FnSigNoRet => {
                let [_, id_node, _, param_list_node, _] = self.expect_children::<5>(node)?;

                let param_list = self.lower_param_list(param_list_node)?;
                let name = self.lower_ident(id_node)?;

                Ok(FnSig {
                    name,
                    params: param_list,
                    ret_ty: None,
                    variadic: false,
                })
            }
            ProdTag::FnSigRetTy => {
                let [_, id_node, _, param_list_node, _, _, ty_node] =
                    self.expect_children::<7>(node)?;

                let param_list = self.lower_param_list(param_list_node)?;
                let name = self.lower_ident(id_node)?;
                let ty = self.lower_ty(ty_node)?;

                Ok(FnSig {
                    name,
                    params: param_list,
                    ret_ty: Some(ty),
                    variadic: false,
                })
            }
            _ => Err(self.unexpected_tag(node, "FnSigNoRet, FnSigRetTy")),
        }
    }

    fn lower_param_list(&mut self, node: CSTNodeID) -> LowerResult<Vec<Param>> {
        let mut param_list = vec![];
        let mut cur_node = node;

        loop {
            let tag = self.get_prod_tag(cur_node)?;
            match tag {
                ProdTag::ParamListEmpty => break,
                ProdTag::ParamListParam => {
                    let [param_node] = self.expect_children::<1>(cur_node)?;
                    param_list.push(self.lower_param(param_node)?);
                    break;
                }
                ProdTag::ParamListParamList => {
                    let [param_node, _, next_node] = self.expect_children::<3>(cur_node)?;
                    param_list.push(self.lower_param(param_node)?);
                    cur_node = next_node;
                }
                _ => return Err(self.unexpected_tag(cur_node, "Empty, Param, ParamList")),
            };
        }

        Ok(param_list)
    }

    fn lower_extern_param_list(&mut self, node: CSTNodeID) -> LowerResult<(Vec<Param>, bool)> {
        let mut params = vec![];
        let mut cur_node = node;
        let mut variadic = false;

        loop {
            let tag = self.get_prod_tag(cur_node)?;
            match tag {
                ProdTag::ExternParamListEmpty => break,
                ProdTag::ExternParamListVariadic => {
                    variadic = true;
                    break;
                }
                ProdTag::ExternParamListParam => {
                    let [param_node] = self.expect_children::<1>(cur_node)?;
                    params.push(self.lower_param(param_node)?);
                    break;
                }
                ProdTag::ExternParamListParamList => {
                    let [param_node, _, next_node] = self.expect_children::<3>(cur_node)?;
                    params.push(self.lower_param(param_node)?);
                    cur_node = next_node;
                }
                _ => {
                    return Err(self.unexpected_tag(cur_node, "Empty, Param, ParamList, Variadic"));
                }
            };
        }

        Ok((params, variadic))
    }

    fn lower_param(&mut self, node: CSTNodeID) -> LowerResult<Param> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::ParamVarAttrIdentTy => {
                let [attr_node, id_node, _, ty_node] = self.expect_children::<4>(node)?;

                let mutable = self.lower_var_attr(attr_node)?;
                let name = self.lower_ident(id_node)?;
                let ty = self.lower_ty(ty_node)?;

                Ok(Param { mutable, name, ty })
            }
            _ => return Err(self.unexpected_tag(node, "VarAttr Ident Ty")),
        }
    }

    fn lower_var_attr(&mut self, node: CSTNodeID) -> LowerResult<bool> {
        let tag = self.get_prod_tag(node)?;
        match tag {
            ProdTag::VarAttrEmpty => Ok(false),
            ProdTag::VarAttrMut => Ok(true),
            _ => return Err(self.unexpected_tag(node, "Empty, Mut")),
        }
    }

    fn lower_block(&mut self, node: CSTNodeID) -> LowerResult<Block> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::BlockStmtList => {
                let [_, stmt_list_node, _] = self.expect_children::<3>(node)?;

                let stmt_list = self.lower_stmt_list(stmt_list_node)?;

                Ok(self.make_block(
                    BlockKind {
                        stmts: stmt_list,
                        tail_expr: None,
                    },
                    span,
                ))
            }
            ProdTag::BlockExpr => {
                let [_, block_expr_body_node, _] = self.expect_children(node)?;

                self.lower_block_expr_body(block_expr_body_node)
            }
            _ => return Err(self.unexpected_tag(node, "StmtList")),
        }
    }

    fn lower_block_expr_body(&mut self, node: CSTNodeID) -> LowerResult<Block> {
        let span = self.get_node_span(node);
        let mut cur_node = node;
        let mut block_expr = BlockKind {
            stmts: vec![],
            tail_expr: None,
        };

        loop {
            let tag = self.get_prod_tag(cur_node)?;
            match tag {
                ProdTag::BlockExprBodyExpr => {
                    let [expr_node] = self.expect_children(cur_node)?;
                    let expr = self.lower_expr(expr_node)?;

                    block_expr.tail_expr = Some(Box::new(expr));
                    break;
                }
                ProdTag::BlockExprBodyStmt => {
                    let [stmt_node, next_node] = self.expect_children(cur_node)?;
                    let stmt = self.lower_stmt(stmt_node)?;
                    block_expr.stmts.push(stmt);

                    cur_node = next_node;
                }
                _ => return Err(self.unexpected_tag(node, "StmtList")),
            }
        }
        Ok(self.make_block(block_expr, span))
    }

    fn lower_stmt_list(&mut self, node: CSTNodeID) -> LowerResult<Vec<Stmt>> {
        let mut cur_node = node;
        let mut stmt_list = vec![];

        loop {
            let tag = self.get_prod_tag(cur_node)?;
            match tag {
                ProdTag::StmtListEmpty => break,
                ProdTag::StmtListStmtStmtList => {
                    let [stmt_node, next_node] = self.expect_children::<2>(cur_node)?;
                    stmt_list.push(self.lower_stmt(stmt_node)?);
                    cur_node = next_node;
                }
                _ => return Err(self.unexpected_tag(cur_node, "Empty, Stmt StmtList")),
            }
        }

        Ok(stmt_list)
    }

    fn lower_stmt(&mut self, node: CSTNodeID) -> LowerResult<Stmt> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        let kind = match tag {
            ProdTag::StmtEmpty => StmtKind::Empty,
            ProdTag::StmtReturn => {
                let [ret_node] = self.expect_children::<1>(node)?;
                let ret_tag = self.get_prod_tag(ret_node)?;
                match ret_tag {
                    ProdTag::ReturnStmtEmpty => StmtKind::Return(None),
                    ProdTag::ReturnStmtExpr => {
                        let [_, expr_node, _] = self.expect_children::<3>(ret_node)?;
                        let expr = self.lower_expr(expr_node)?;
                        StmtKind::Return(Some(expr))
                    }
                    _ => return Err(self.unexpected_tag(ret_node, "Empty, ")),
                }
            }
            ProdTag::StmtVarDecl => {
                let [var_decl_node] = self.expect_children::<1>(node)?;
                let var_decl_tag = self.get_prod_tag(var_decl_node)?;
                assert_eq!(var_decl_tag, ProdTag::VarDeclStmt);
                let [_, var_decl_node, _] = self.expect_children::<3>(var_decl_node)?;
                let decl = self.lower_var_decl(var_decl_node)?;
                StmtKind::Let {
                    mutable: decl.mutable,
                    name: decl.name,
                    ty: decl.ty,
                    init: None,
                }
            }
            ProdTag::StmtVarInit => {
                let [var_init_node] = self.expect_children::<1>(node)?;
                let var_init_tag = self.get_prod_tag(var_init_node)?;
                assert_eq!(var_init_tag, ProdTag::VarInitStmt);

                let [_, var_decl_node, _, expr_node, _] =
                    self.expect_children::<5>(var_init_node)?;
                let decl = self.lower_var_decl(var_decl_node)?;
                let expr = self.lower_expr(expr_node)?;
                StmtKind::Let {
                    mutable: decl.mutable,
                    name: decl.name,
                    ty: decl.ty,
                    init: Some(expr),
                }
            }
            ProdTag::StmtAssign => {
                let [assign_stmt_node] = self.expect_children::<1>(node)?;
                let assign_stmt_tag = self.get_prod_tag(assign_stmt_node)?;

                assert_eq!(assign_stmt_tag, ProdTag::AssignStmt);

                let [l_val_node, _, expr_node, _] = self.expect_children::<4>(assign_stmt_node)?;
                let target = self.lower_place(l_val_node)?;
                let value = self.lower_expr(expr_node)?;

                StmtKind::Assign { target, value }
            }
            ProdTag::StmtExpr => {
                let [expr_node, _] = self.expect_children::<2>(node)?;

                let expr = self.lower_expr(expr_node)?;

                StmtKind::Semi(expr)
            }
            ProdTag::StmtIf => {
                let [if_stmt_node] = self.expect_children::<1>(node)?;
                self.lower_if_stmt(if_stmt_node)?
            }
            ProdTag::StmtLoop => {
                let [loop_stmt_node] = self.expect_children::<1>(node)?;
                self.lower_loop_stmt(loop_stmt_node)?
            }
            ProdTag::StmtBreak => StmtKind::Break(None),
            ProdTag::StmtContinue => StmtKind::Continue,
            ProdTag::StmtBreakExpr => {
                let [_, expr_node, _] = self.expect_children(node)?;
                let expr = self.lower_expr(expr_node)?;

                StmtKind::Break(Some(expr))
            }
            _ => {
                return Err(self.unexpected_tag(
                    node,
                    "Return, VarDecl, VarInit, Assign, Expr, If, Loop, Break, Continue, BreakExpr",
                ));
            }
        };

        Ok(AstNode {
            id: self.ids.alloc(),
            kind,
            span,
        })
    }

    fn lower_var_decl(&mut self, node: CSTNodeID) -> LowerResult<VarDecl> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::VarDeclNoTy => {
                let [var_attr_node, id_node] = self.expect_children::<2>(node)?;

                let mutable = self.lower_var_attr(var_attr_node)?;
                let name = self.lower_ident(id_node)?;

                Ok(VarDecl {
                    mutable,
                    name,
                    ty: None,
                })
            }
            ProdTag::VarDeclWithTy => {
                let [var_attr_node, id_node, _, ty_node] = self.expect_children::<4>(node)?;

                let mutable = self.lower_var_attr(var_attr_node)?;
                let name = self.lower_ident(id_node)?;
                let ty = self.lower_ty(ty_node)?;

                Ok(VarDecl {
                    mutable,
                    name,
                    ty: Some(ty),
                })
            }
            _ => return Err(self.unexpected_tag(node, "VarDeclNoTy, VarDeclWithTy")),
        }
    }

    fn lower_expr_list(&mut self, node: CSTNodeID) -> LowerResult<Vec<Expr>> {
        let mut cur_node = node;
        let mut expr_list = vec![];

        loop {
            let tag = self.get_prod_tag(cur_node)?;
            match tag {
                ProdTag::ArrayElemListEmpty => break,
                ProdTag::ArrayElemListExpr => {
                    let [expr_node] = self.expect_children(cur_node)?;
                    expr_list.push(self.lower_expr(expr_node)?);
                    break;
                }
                ProdTag::ArrayElemListExprList => {
                    let [expr_node, _, next_node] = self.expect_children(cur_node)?;
                    expr_list.push(self.lower_expr(expr_node)?);
                    cur_node = next_node;
                }
                _ => return Err(self.unexpected_tag(cur_node, "ExprAdd, ExprCmp")),
            }
        }

        Ok(expr_list)
    }

    fn lower_tuple_expr_inner(&mut self, node: CSTNodeID) -> LowerResult<Vec<Expr>> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::TupleExprInnerEmpty => Ok(vec![]),
            ProdTag::TupleExprInnerExpr => {
                let [expr_node, _, tuple_elem_list_node] = self.expect_children::<3>(node)?;
                let mut expr_list = vec![self.lower_expr(expr_node)?];
                expr_list.append(&mut self.lower_tuple_elem_list(tuple_elem_list_node)?);

                Ok(expr_list)
            }
            _ => Err(self.unexpected_tag(node, "TupleExprInnerEmpty, TupleExprInnerExpr")),
        }
    }

    fn lower_tuple_elem_list(&mut self, node: CSTNodeID) -> LowerResult<Vec<Expr>> {
        let mut cur_node = node;
        let mut expr_list = vec![];

        loop {
            let tag = self.get_prod_tag(cur_node)?;
            match tag {
                ProdTag::TupleElemListEmpty => break,
                ProdTag::TupleElemListExpr => {
                    let [expr_node] = self.expect_children::<1>(cur_node)?;
                    expr_list.push(self.lower_expr(expr_node)?);
                    break;
                }
                ProdTag::TupleElemListExprList => {
                    let [expr_node, _, next_node] = self.expect_children::<3>(cur_node)?;
                    expr_list.push(self.lower_expr(expr_node)?);
                    cur_node = next_node;
                }
                _ => return Err(self.unexpected_tag(cur_node, "Empty, Expr, Expr List")),
            }
        }

        Ok(expr_list)
    }

    fn lower_expr(&mut self, node: CSTNodeID) -> LowerResult<Expr> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::ExprAdd => {
                let [add_expr_node] = self.expect_children::<1>(node)?;
                Ok(self.lower_add_expr(add_expr_node)?)
            }
            ProdTag::ExprCmp => {
                let [expr_node, cmp_op_node, add_expr] = self.expect_children::<3>(node)?;
                let lhs = Box::new(self.lower_expr(expr_node)?);
                let rhs = Box::new(self.lower_add_expr(add_expr)?);
                let op = self.lower_binary_op(cmp_op_node)?;

                let kind = ExprKind::Binary { op, lhs, rhs };

                Ok(self.make_expr(kind, span))
            }
            ProdTag::RangeExpr => {
                let [expr_left, _, expr_right] = self.expect_children::<3>(node)?;
                let start = Box::new(self.lower_expr(expr_left)?);
                let end = Box::new(self.lower_expr(expr_right)?);

                let kind = ExprKind::Range { start, end };

                Ok(self.make_expr(kind, span))
            }
            ProdTag::IterExpr => {
                let [expr_node] = self.expect_children(node)?;

                Ok(self.lower_expr(expr_node)?)
            }
            _ => return Err(self.unexpected_tag(node, "ExprAdd, ExprCmp")),
        }
    }

    fn lower_binary_op(&mut self, node: CSTNodeID) -> LowerResult<BinaryOp> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::CmpOpEq => Ok(BinaryOp::Eq),
            ProdTag::CmpOpGe => Ok(BinaryOp::Ge),
            ProdTag::CmpOpGt => Ok(BinaryOp::Gt),
            ProdTag::CmpOpLe => Ok(BinaryOp::Le),
            ProdTag::CmpOpLt => Ok(BinaryOp::Lt),
            ProdTag::CmpOpNe => Ok(BinaryOp::Ne),
            ProdTag::AddOpPlus => Ok(BinaryOp::Add),
            ProdTag::AddOpMinus => Ok(BinaryOp::Sub),
            ProdTag::MulOpStar => Ok(BinaryOp::Mul),
            ProdTag::MulOpSlash => Ok(BinaryOp::Div),
            _ => {
                return Err(self.unexpected_tag(
                    node,
                    "'<' | '<=' | '>' | '>=' | '==' | '!=' | '+' | '-' | '*' | '/'",
                ));
            }
        }
    }

    fn lower_add_expr(&mut self, node: CSTNodeID) -> LowerResult<Expr> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::AddExprTerm => {
                let [term_node] = self.expect_children::<1>(node)?;
                self.lower_term(term_node)
            }
            ProdTag::AddExprBinary => {
                let [add_expr_node, add_op, term_node] = self.expect_children::<3>(node)?;

                let lhs = Box::new(self.lower_add_expr(add_expr_node)?);
                let rhs = Box::new(self.lower_term(term_node)?);
                let op = self.lower_binary_op(add_op)?;

                let kind = ExprKind::Binary { op, lhs, rhs };

                Ok(self.make_expr(kind, span))
            }
            _ => return Err(self.unexpected_tag(node, "AddExprBinary")),
        }
    }

    fn lower_term(&mut self, node: CSTNodeID) -> LowerResult<Expr> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::TermBinary => {
                let [term_node, mul_op_node, factor_node] = self.expect_children::<3>(node)?;

                let lhs = Box::new(self.lower_term(term_node)?);
                let rhs = Box::new(self.lower_factor(factor_node)?);
                let op = self.lower_binary_op(mul_op_node)?;

                let kind = ExprKind::Binary { op, lhs, rhs };

                Ok(self.make_expr(kind, span))
            }
            ProdTag::TermFactor => {
                let [factor_node] = self.expect_children::<1>(node)?;
                self.lower_factor(factor_node)
            }
            _ => return Err(self.unexpected_tag(node, "TermBinary, TermFactor")),
        }
    }

    fn lower_factor(&mut self, node: CSTNodeID) -> LowerResult<Expr> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::FactorNum => {
                let [num_node] = self.expect_children::<1>(node)?;

                self.lower_num(num_node)
            }
            ProdTag::FactorString => {
                let [literal_string_node] = self.expect_children::<1>(node)?;
                let value = self.lower_string_literal(literal_string_node)?;

                Ok(self.make_expr(ExprKind::String(value), span))
            }
            ProdTag::FactorLVal => {
                let [l_val_node] = self.expect_children::<1>(node)?;

                self.lower_l_val_expr(l_val_node)
            }
            ProdTag::FactorGroupedExpr => {
                let [_, expr_node, _] = self.expect_children::<3>(node)?;

                self.lower_expr(expr_node)
            }
            ProdTag::FactorCall => {
                let [id_node, _, arg_list_node, _] = self.expect_children::<4>(node)?;

                let callee = self.lower_ident(id_node)?;
                let args = self.lower_arg_list(arg_list_node)?;

                let kind = ExprKind::Call { callee, args };

                Ok(self.make_expr(kind, span))
            }
            _ => return Err(self.unexpected_tag(node, "Num, String, LVal, GroupedExpr, Call")),
        }
    }

    fn lower_string_literal(&mut self, node: CSTNodeID) -> LowerResult<String> {
        let raw = self.get_token_text(node)?;
        let span = self.get_node_span(node);
        let Some(inner) = raw
            .strip_prefix('"')
            .and_then(|text| text.strip_suffix('"'))
        else {
            return Err(LowerError {
                message: format!("Invalid string literal: {raw}."),
                span,
            });
        };

        let mut chars = inner.chars();
        let mut value = String::new();
        while let Some(ch) = chars.next() {
            if ch != '\\' {
                value.push(ch);
                continue;
            }

            let Some(escaped) = chars.next() else {
                return Err(LowerError {
                    message: "String literal ends with an incomplete escape.".into(),
                    span,
                });
            };
            match escaped {
                'n' => value.push('\n'),
                't' => value.push('\t'),
                '\\' => value.push('\\'),
                '"' => value.push('"'),
                '0' => value.push('\0'),
                _ => {
                    return Err(LowerError {
                        message: format!("Invalid string escape: \\{escaped}."),
                        span,
                    });
                }
            }
        }

        Ok(value)
    }

    fn lower_arg_list(&mut self, node: CSTNodeID) -> LowerResult<Vec<Expr>> {
        let mut cur_node = node;
        let mut arg_list = vec![];

        loop {
            let tag = self.get_prod_tag(cur_node)?;
            match tag {
                ProdTag::ArgListEmpty => break,
                ProdTag::ArgListExpr => {
                    let [expr_node] = self.expect_children::<1>(cur_node)?;
                    arg_list.push(self.lower_expr(expr_node)?);
                    break;
                }
                ProdTag::ArgListExprList => {
                    let [expr_node, _, next_node] = self.expect_children::<3>(cur_node)?;
                    arg_list.push(self.lower_expr(expr_node)?);
                    cur_node = next_node;
                }
                _ => return Err(self.unexpected_tag(cur_node, "Empty, Expr, Expr List")),
            }
        }

        Ok(arg_list)
    }

    fn lower_if_stmt(&mut self, node: CSTNodeID) -> LowerResult<StmtKind> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::IfStmt => {
                let [_, expr_node, block_node, else_part_node] = self.expect_children::<4>(node)?;

                let cond = self.lower_expr(expr_node)?;
                let then_block = self.lower_block(block_node)?;
                let else_branch = self.lower_else_branch(else_part_node)?;

                Ok(StmtKind::If {
                    cond,
                    then_block,
                    else_branch,
                })
            }
            _ => return Err(self.unexpected_tag(node, "IfStmt")),
        }
    }

    fn lower_else_branch(&mut self, node: CSTNodeID) -> LowerResult<Option<ElseBranch>> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::ElsePartEmpty => Ok(None),
            ProdTag::ElsePartBlock => {
                let [_, block_node] = self.expect_children::<2>(node)?;
                let block = self.lower_block(block_node)?;

                Ok(Some(ElseBranch::Block(block)))
            }
            ProdTag::ElsePartIf => {
                let [_, _, expr_node, block_node, else_part_node] =
                    self.expect_children::<5>(node)?;

                let cond = self.lower_expr(expr_node)?;
                let then_block = self.lower_block(block_node)?;
                let else_branch = self.lower_else_branch(else_part_node)?.map(Box::new);

                Ok(Some(ElseBranch::If {
                    cond,
                    then_block,
                    else_branch,
                }))
            }
            _ => return Err(self.unexpected_tag(node, "Empty, Block, If")),
        }
    }

    fn lower_loop_stmt(&mut self, node: CSTNodeID) -> LowerResult<StmtKind> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::LoopStmtWhile => {
                let [while_stmt_node] = self.expect_children::<1>(node)?;
                self.lower_while_stmt(while_stmt_node)
            }
            ProdTag::LoopStmtFor => {
                let [for_stmt_node] = self.expect_children::<1>(node)?;
                self.lower_for_stmt(for_stmt_node)
            }
            ProdTag::LoopStmtInfinite => {
                let [loop_inf_stmt_node] = self.expect_children::<1>(node)?;
                self.lower_loop_inf_stmt(loop_inf_stmt_node)
            }
            _ => return Err(self.unexpected_tag(node, "Empty, Block, If")),
        }
    }

    fn lower_while_stmt(&mut self, node: CSTNodeID) -> LowerResult<StmtKind> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::WhileStmt => {
                let [_, expr_node, block_node] = self.expect_children::<3>(node)?;

                let cond = self.lower_expr(expr_node)?;
                let body = self.lower_block(block_node)?;

                Ok(StmtKind::While { cond, body })
            }
            _ => return Err(self.unexpected_tag(node, "WhileStmt")),
        }
    }

    fn lower_for_stmt(&mut self, node: CSTNodeID) -> LowerResult<StmtKind> {
        let tag = self.get_prod_tag(node)?;
        match tag {
            ProdTag::ForStmt => {
                let [_, var_decl_node, _, range_node, block_node] =
                    self.expect_children::<5>(node)?;

                let decl = self.lower_var_decl(var_decl_node)?;
                let range = self.lower_expr(range_node)?;
                let block = self.lower_block(block_node)?;

                Ok(StmtKind::For {
                    mutable: decl.mutable,
                    var: decl.name,
                    ty: decl.ty,
                    iter: range,
                    body: block,
                })
            }
            _ => return Err(self.unexpected_tag(node, "ForStmt")),
        }
    }

    fn lower_loop_inf_stmt(&mut self, node: CSTNodeID) -> LowerResult<StmtKind> {
        let tag = self.get_prod_tag(node)?;
        match tag {
            ProdTag::InfiniteLoopStmt => {
                let [_, block_node] = self.expect_children::<2>(node)?;

                let block = Box::new(self.lower_block(block_node)?);

                Ok(StmtKind::Loop { body: block })
            }
            _ => return Err(self.unexpected_tag(node, "InfLoopStmt")),
        }
    }

    fn lower_num(&mut self, node: CSTNodeID) -> LowerResult<Expr> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);
        match tag {
            ProdTag::NumLiteralI32 => {
                let [literal_i32_node] = self.expect_children(node)?;
                let literal = self.get_token_text(literal_i32_node)?;
                let value = literal.trim().parse::<i32>().map_err(|err| LowerError {
                    message: format!(
                        "Invalid i32 literal: {literal}, internal error message: {err}"
                    ),
                    span: span.clone(),
                })?;
                Ok(AstNode {
                    id: self.ids.alloc(),
                    kind: ExprKind::Int(value),
                    span,
                })
            }
            _ => return Err(self.unexpected_tag(node, "i32")),
        }
    }

    /// ## 基本说明
    /// 该函数在右侧表达式中推导出“左值”时使用，会解析外层的&或&mut引用标记，并将可取位置以正确形式包裹在表达式中
    ///
    /// 而当在赋值表达式左侧使用“左值”时，直接调用place，因为表达式不能被赋值
    ///
    /// ## 解释
    /// 这样的分层使得该阶段可以间接处理文法定义中的缺陷：即左侧可能出现带借用的值，这源于文法定义中对“左值”概念的模糊使用
    fn lower_l_val_expr(&mut self, node: CSTNodeID) -> LowerResult<Expr> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::LValAddr => {
                let [addr_node] = self.expect_children(node)?;

                self.lower_addr_expr(addr_node)
            }
            ProdTag::LValDeref => {
                let place = self.lower_place(node)?;
                Ok(self.make_expr(ExprKind::Place(place), span))
            }
            _ => return Err(self.unexpected_tag(node, "i32")),
        }
    }

    fn lower_addr_expr(&mut self, node: CSTNodeID) -> LowerResult<Expr> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);
        match tag {
            ProdTag::AddrAddrElem => {
                let [addr_elem_node] = self.expect_children(node)?;
                self.lower_addr_elem_expr(addr_elem_node)
            }
            ProdTag::AddrRef => {
                let [_, addr_node] = self.expect_children(node)?;

                let expr = Box::new(self.lower_addr_expr(addr_node)?);

                let kind = ExprKind::Borrow {
                    mutable: false,
                    expr,
                };

                Ok(self.make_expr(kind, span))
            }
            ProdTag::AddrRefMut => {
                let [_, _, addr_node] = self.expect_children(node)?;

                let expr = Box::new(self.lower_addr_expr(addr_node)?);

                let kind = ExprKind::Borrow {
                    mutable: true,
                    expr,
                };

                Ok(self.make_expr(kind, span))
            }
            _ => return Err(self.unexpected_tag(node, "AddrElem, Ref, RefMut")),
        }
    }

    fn lower_addr_elem_expr(&mut self, node: CSTNodeID) -> LowerResult<Expr> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::AddrElemIdent => {
                let [ident_node] = self.expect_children(node)?;
                let name = self.lower_ident(ident_node)?;

                let kind = PlaceKind::Local(name);
                let place = self.make_place(kind, span.clone());

                let kind = ExprKind::Place(place);
                Ok(self.make_expr(kind, span))
            }
            ProdTag::AddrElemBlockExpr => {
                let [block_expr] = self.expect_children(node)?;

                let block = self.lower_block(block_expr)?;
                let kind = ExprKind::Block(Box::new(block));

                Ok(self.make_expr(kind, span))
            }
            ProdTag::AddrElemBranchExpr => {
                let [branch_expr_node] = self.expect_children::<1>(node)?;
                let [_, expr_node, block_expr_node1, _, block_expr_node2] =
                    self.expect_children::<5>(branch_expr_node)?;

                let cond = Box::new(self.lower_expr(expr_node)?);
                let then_block = Box::new(self.lower_block(block_expr_node1)?);
                let else_block = Box::new(self.lower_block(block_expr_node2)?);

                let kind = ExprKind::If {
                    cond,
                    then_block,
                    else_block,
                };

                Ok(self.make_expr(kind, span))
            }
            ProdTag::AddrElemLoopExpr => {
                let [loop_expr_node] = self.expect_children::<1>(node)?;
                let [_, block_expr_node] = self.expect_children::<2>(loop_expr_node)?;

                let body = Box::new(self.lower_block(block_expr_node)?);

                let kind = ExprKind::Loop { body };

                Ok(self.make_expr(kind, span))
            }
            ProdTag::AddrElemArray => {
                let [_, array_elem_list_node, _] = self.expect_children::<3>(node)?;

                let array_elem_list = self.lower_expr_list(array_elem_list_node)?;

                let kind = ExprKind::Array(array_elem_list);

                Ok(self.make_expr(kind, span))
            }
            ProdTag::AddrElemTuple => {
                let [_, tuple_expr_inner_node, _] = self.expect_children::<3>(node)?;
                let elem_list = self.lower_tuple_expr_inner(tuple_expr_inner_node)?;

                Ok(self.make_expr(ExprKind::Tuple(elem_list), span))
            }
            ProdTag::AddrElemIndex => {
                let [addr_elem_node, _, expr_node, _] = self.expect_children::<4>(node)?;
                let base = Box::new(self.lower_addr_elem_expr(addr_elem_node)?);
                let index = Box::new(self.lower_expr(expr_node)?);

                Ok(self.make_expr(ExprKind::Index { base, index }, span))
            }
            ProdTag::AddrElemField => {
                let place = self.lower_addr_elem_as_place(node)?;
                Ok(self.make_expr(ExprKind::Place(place), span))
            }
            _ => return Err(self.unexpected_tag(node, "AddrElem, Ref, RefMut")),
        }
    }

    fn lower_place(&mut self, node: CSTNodeID) -> LowerResult<Place> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::LValAddr => {
                let [addr_node] = self.expect_children::<1>(node)?;
                self.lower_addr_as_place(addr_node)
            }
            ProdTag::LValDeref => {
                let [_, l_val_node] = self.expect_children::<2>(node)?;
                let expr = self.lower_l_val_expr(l_val_node)?;

                Ok(self.make_place(PlaceKind::Deref(Box::new(expr)), span))
            }
            _ => Err(self.unexpected_tag(node, "LValAddr, LValDeref")),
        }
    }

    fn lower_addr_as_place(&mut self, node: CSTNodeID) -> LowerResult<Place> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::AddrAddrElem => {
                let [addr_elem_node] = self.expect_children::<1>(node)?;
                self.lower_addr_elem_as_place(addr_elem_node)
            }
            ProdTag::AddrRef | ProdTag::AddrRefMut => {
                Err(self.unexpected_tag(node, "place without borrow prefix"))
            }
            _ => Err(self.unexpected_tag(node, "AddrElem")),
        }
    }

    fn lower_addr_elem_as_place(&mut self, node: CSTNodeID) -> LowerResult<Place> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::AddrElemIdent => {
                let [id_node] = self.expect_children(node)?;
                let ident = self.lower_ident(id_node)?;

                let kind = PlaceKind::Local(ident);

                Ok(self.make_place(kind, span))
            }
            ProdTag::AddrElemIndex => {
                let [addr_elem_node, _, expr_node, _] = self.expect_children::<4>(node)?;
                let base = Box::new(self.lower_addr_elem_as_place(addr_elem_node)?);
                let index = Box::new(self.lower_expr(expr_node)?);

                Ok(self.make_place(PlaceKind::Index { base, index }, span))
            }
            ProdTag::AddrElemField => {
                let [addr_elem_node, _, num_node] = self.expect_children::<3>(node)?;
                let base = Box::new(self.lower_addr_elem_as_place(addr_elem_node)?);
                let num = self.lower_num(num_node)?;

                let index = match num.kind {
                    ExprKind::Int(index) if index >= 0 => index as usize,
                    _ => {
                        return Err(LowerError {
                            message: "Tuple field index must be a non-negative integer.".into(),
                            span: num.span,
                        });
                    }
                };

                Ok(self.make_place(PlaceKind::Field { base, index }, span))
            }
            _ => return Err(self.unexpected_tag(node, "AddrElem, Ref, RefMut")),
        }
    }

    fn lower_ty_list(&mut self, node: CSTNodeID) -> LowerResult<Vec<Ty>> {
        let mut cur_node = node;
        let mut ty_list = vec![];

        loop {
            let tag = self.get_prod_tag(cur_node)?;

            match tag {
                ProdTag::TyListEmpty => break,
                ProdTag::TyListTy => {
                    let [ty_node] = self.expect_children(cur_node)?;
                    ty_list.push(self.lower_ty(ty_node)?);
                    break;
                }
                ProdTag::TyListTyList => {
                    let [ty_node, _, next_node] = self.expect_children(cur_node)?;
                    ty_list.push(self.lower_ty(ty_node)?);
                    cur_node = next_node;
                }
                _ => return Err(self.unexpected_tag(node, "AddrElem, Ref, RefMut")),
            }
        }

        Ok(ty_list)
    }

    fn lower_ty(&mut self, node: CSTNodeID) -> LowerResult<Ty> {
        let tag = self.get_prod_tag(node)?;
        let span = self.get_node_span(node);

        match tag {
            ProdTag::TyI32 => Ok(self.make_ty(TyKind::I32, span)),
            ProdTag::TyStr => Ok(self.make_ty(TyKind::Str, span)),
            ProdTag::TyArray => {
                let [_, ty_node, _, num_node, _] = self.expect_children(node)?;

                let ty = self.lower_ty(ty_node)?;
                let num = self.lower_num(num_node)?;

                let len = match num.kind {
                    ExprKind::Int(len) => len,
                    _ => return Err(self.unexpected_tag(node, "i32")),
                };

                let kind = TyKind::Array {
                    elem: Box::new(ty),
                    len: len as usize,
                };

                Ok(self.make_ty(kind, span))
            }
            ProdTag::TyRef => {
                let [_, ty_node] = self.expect_children(node)?;

                let inner = Box::new(self.lower_ty(ty_node)?);

                let kind = TyKind::Ref {
                    mutable: false,
                    inner,
                };

                Ok(self.make_ty(kind, span))
            }
            ProdTag::TyRefMut => {
                let [_, _, ty_node] = self.expect_children(node)?;

                let inner = Box::new(self.lower_ty(ty_node)?);

                let kind = TyKind::Ref {
                    mutable: true,
                    inner,
                };

                Ok(self.make_ty(kind, span))
            }
            ProdTag::TyTuple => {
                let [_, tuple_ty_inner_node, _] = self.expect_children(node)?;

                let ty_list = self.lower_tuple(tuple_ty_inner_node)?;

                let kind = TyKind::Tuple(ty_list);

                Ok(self.make_ty(kind, span))
            }
            _ => return Err(self.unexpected_tag(node, "I32, Str, Array, Ref, RefMut, Tuple")),
        }
    }

    fn lower_tuple(&mut self, node: CSTNodeID) -> LowerResult<Vec<Ty>> {
        let tag = self.get_prod_tag(node)?;

        match tag {
            ProdTag::TupleTyInnerEmpty => Ok(vec![]),
            ProdTag::TupleTyInnerTy => {
                let [ty_node, _, ty_list_node] = self.expect_children(node)?;
                let ty = self.lower_ty(ty_node)?;
                let mut ty_list = self.lower_ty_list(ty_list_node)?;
                ty_list.insert(0, ty);

                Ok(ty_list)
            }
            _ => return Err(self.unexpected_tag(node, "I32, Array, Ref, RefMut, Tuple")),
        }
    }

    fn lower_ident(&mut self, node: CSTNodeID) -> LowerResult<Ident> {
        let name = self.get_token_text(node)?;
        let span = self.get_node_span(node);
        Ok(AstNode::new(self.ids.alloc(), name, span))
    }

    fn get_node_span(&self, node: CSTNodeID) -> Span {
        match self.cst.node(node) {
            CSTNode::Rule(n) => n.span.clone(),
            CSTNode::Token(n) => n.span.clone(),
        }
    }

    fn get_children(&self, node: CSTNodeID) -> LowerResult<&[CSTNodeID]> {
        match self.cst.node(node) {
            CSTNode::Rule(n) => Ok(n.children.as_slice()),
            CSTNode::Token(n) => Err(LowerError {
                message: format!("Node {node:?} has no child."),
                span: n.span.clone(),
            }),
        }
    }

    fn get_token_text(&self, node: CSTNodeID) -> LowerResult<String> {
        match self.cst.node(node) {
            CSTNode::Token(n) => {
                if let Some(text) = self.source.slice(n.span.clone()) {
                    Ok(text.to_string())
                } else {
                    Err(LowerError {
                        message: format!("Fail to get span of node {node:?} form sourcefile."),
                        span: n.span.clone(),
                    })
                }
            }
            CSTNode::Rule(n) => Err(LowerError {
                message: format!("Expected token, found rule node {node:?}."),
                span: n.span.clone(),
            }),
        }
    }

    fn get_production_id(&self, node: CSTNodeID) -> LowerResult<ProductionId> {
        match self.cst.node(node) {
            CSTNode::Rule(n) => Ok(n.production),
            CSTNode::Token(n) => Err(LowerError {
                message: format!("Node {node:?} is not production."),
                span: n.span.clone(),
            }),
        }
    }

    fn get_prod_tag(&self, node: CSTNodeID) -> LowerResult<ProdTag> {
        let prod = self.get_production_id(node)?;
        let span = self.get_node_span(node);

        self.ctx.prod_tag(prod).ok_or(LowerError {
            message: format!("Missing production tag for production {prod:?}."),
            span,
        })
    }

    fn unexpected_tag(&self, node: CSTNodeID, expected: &str) -> LowerError {
        LowerError {
            message: format!(
                "Unexpected production tag {:?} on node {:?}, expected {}.",
                self.get_prod_tag(node).ok(),
                node,
                expected,
            ),
            span: self.get_node_span(node),
        }
    }

    fn expect_children<const N: usize>(&self, node: CSTNodeID) -> LowerResult<[CSTNodeID; N]> {
        let children = self.get_children(node)?;

        children.try_into().map_err(|_| LowerError {
            message: format!("expected {N} children, found {}", children.len()),
            span: self.get_node_span(node),
        })
    }

    fn make_item(&mut self, kind: ItemKind, span: Span) -> Item {
        AstNode::new(self.ids.alloc(), kind, span)
    }

    fn make_expr(&mut self, kind: ExprKind, span: Span) -> Expr {
        AstNode::new(self.ids.alloc(), kind, span)
    }

    #[allow(dead_code)]
    fn make_stmt(&mut self, kind: StmtKind, span: Span) -> Stmt {
        AstNode::new(self.ids.alloc(), kind, span)
    }

    fn make_block(&mut self, kind: BlockKind, span: Span) -> Block {
        AstNode::new(self.ids.alloc(), kind, span)
    }

    fn make_place(&mut self, kind: PlaceKind, span: Span) -> Place {
        AstNode::new(self.ids.alloc(), kind, span)
    }

    fn make_ty(&mut self, kind: TyKind, span: Span) -> Ty {
        AstNode::new(self.ids.alloc(), kind, span)
    }
}
