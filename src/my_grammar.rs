use crate::parser::ProductionId;
use crate::parser::grammar::{Grammar, GrammarBuilder, GrammarBuilderErr};
use crate::parser::symbol::{NonTerminalId, Symbol, TerminalId};
use serde::{Deserialize, Serialize};
/// 方便转换为Symbol，避免into调用过于冗长

macro_rules! rhs {
    ($($x:expr),* $(,)?) => {
        [$( $x.into() ),*]
    };
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Terminals {
    pub i8_: TerminalId,
    pub i16_: TerminalId,
    pub i32_: TerminalId,
    pub i64_: TerminalId,
    pub u8_: TerminalId,
    pub u16_: TerminalId,
    pub u32_: TerminalId,
    pub u64_: TerminalId,
    pub usize_: TerminalId,
    pub isize_: TerminalId,
    pub bool_: TerminalId,
    pub true_: TerminalId,
    pub false_: TerminalId,
    pub let_: TerminalId,
    pub if_: TerminalId,
    pub else_: TerminalId,
    pub while_: TerminalId,
    pub return_: TerminalId,
    pub mut_: TerminalId,
    pub fn_: TerminalId,
    pub for_: TerminalId,
    pub in_: TerminalId,
    pub loop_: TerminalId,
    pub break_: TerminalId,
    pub continue_: TerminalId,
    pub extern_: TerminalId,
    pub str_: TerminalId,
    pub struct_: TerminalId,
    pub ident: TerminalId,
    pub literal_i32: TerminalId,
    pub literal_string: TerminalId,
    pub assignment: TerminalId,
    pub plus: TerminalId,
    pub minus: TerminalId,
    pub star: TerminalId,
    pub slash: TerminalId,
    pub eqeq: TerminalId,
    pub gt: TerminalId,
    pub ge: TerminalId,
    pub lt: TerminalId,
    pub le: TerminalId,
    pub ne: TerminalId,
    pub amp: TerminalId,
    pub l_paren: TerminalId,
    pub r_paren: TerminalId,
    pub l_brace: TerminalId,
    pub r_brace: TerminalId,
    pub l_bracket: TerminalId,
    pub r_bracket: TerminalId,
    pub comma: TerminalId,
    pub colon: TerminalId,
    pub semicolon: TerminalId,
    pub arrow: TerminalId,
    pub dot: TerminalId,
    pub dotdot: TerminalId,
    pub ellipsis: TerminalId,
    pub eof: TerminalId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProdTag {
    VarAttrMut,
    TyI8,
    TyI16,
    TyI32,
    TyI64,
    TyU8,
    TyU16,
    TyU32,
    TyU64,
    TyUsize,
    TyIsize,
    TyBool,
    TyStr,
    TyAdt,
    LValAddr,
    AddrAddrElem,
    AddrElemIdent,
    AddrElemIdentTailEmpty,
    AddrElemIdentTailStructLit,
    ProgramDeclList,
    DeclListEmpty,
    DeclListDeclDeclList,
    DeclFnDecl,
    DeclExternFnDecl,
    DeclStructDecl,
    StructDeclNamed,
    StructFieldListEmpty,
    StructFieldListField,
    StructFieldListFieldList,
    StructFieldNamed,
    FnDeclSigBlock,
    ExternFnDeclNoRet,
    ExternFnDeclRetTy,
    FnSigNoRet,
    ExternParamListEmpty,
    ExternParamListParam,
    ExternParamListParamList,
    ExternParamListVariadic,
    ParamListEmpty,
    BlockStmtList,
    StmtListEmpty,
    StmtListStmtStmtList,
    StmtEmpty,
    StmtReturn,
    ReturnStmtEmpty,
    ParamListParam,
    ParamListParamList,
    ParamVarAttrIdentTy,
    FnSigRetTy,
    ReturnStmtExpr,
    VarDeclNoTy,
    VarDeclWithTy,
    LetDeclNoTy,
    LetDeclWithTy,
    PatIdent,
    PatIdentTailEmpty,
    PatIdentTailStruct,
    PatMutIdent,
    PatTuple,
    PatTupleInnerEmpty,
    PatTupleInnerPat,
    PatListEmpty,
    PatListPat,
    PatListPatList,
    StructPatFieldListEmpty,
    StructPatFieldListField,
    StructPatFieldListFieldList,
    StructPatFieldNamed,
    StmtVarDecl,
    VarDeclStmt,
    StmtAssign,
    AssignStmt,
    StmtVarInit,
    VarInitStmt,
    StmtExpr,
    ExprAdd,
    AddExprTerm,
    TermFactor,
    FactorNum,
    FactorTrue,
    FactorFalse,
    FactorString,
    NumLiteralI32,
    FactorLVal,
    FactorGroupedExpr,
    ExprCmp,
    CmpOpLt,
    CmpOpLe,
    CmpOpGt,
    CmpOpGe,
    CmpOpEq,
    CmpOpNe,
    AddExprBinary,
    AddOpPlus,
    AddOpMinus,
    TermBinary,
    MulOpStar,
    MulOpSlash,
    FactorCall,
    FactorStructLit,
    ArgListEmpty,
    ArgListExpr,
    ArgListExprList,
    StructLitFieldListEmpty,
    StructLitFieldListField,
    StructLitFieldListFieldList,
    StructLitFieldNamed,
    StmtIf,
    IfStmt,
    ElsePartEmpty,
    ElsePartBlock,
    ElsePartIf,
    StmtLoop,
    LoopStmtWhile,
    WhileStmt,
    LoopStmtFor,
    ForStmt,
    ForStmtIter,
    RangeExpr,
    LoopStmtInfinite,
    InfiniteLoopStmt,
    StmtBreak,
    StmtContinue,
    VarAttrEmpty,
    TyRef,
    AddrRef,
    TyRefMut,
    AddrRefMut,
    LValDeref,
    BlockExpr,
    BlockExprBodyExpr,
    BlockExprBodyStmt,
    AddrElemBlockExpr,
    FnDeclSigBlockExpr,
    AddrElemBranchExpr,
    BranchExpr,
    AddrElemLoopExpr,
    LoopExpr,
    StmtBreakExpr,
    TyArray,
    AddrElemArray,
    ArrayElemListEmpty,
    ArrayElemListExpr,
    ArrayElemListExprList,
    IterExpr,
    AddrElemIndex,
    TyTuple,
    TupleTyInnerEmpty,
    TupleTyInnerTy,
    TyListEmpty,
    TyListTy,
    TyListTyList,
    AddrElemTuple,
    TupleExprInnerEmpty,
    TupleExprInnerExpr,
    TupleElemListEmpty,
    TupleElemListExpr,
    TupleElemListExprList,
    AddrElemField,
    AddrElemNamedField,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarContext {
    pub grammar: Grammar,
    pub terminals: Terminals,
    pub prod_tags: Vec<Option<ProdTag>>,
}

impl GrammarContext {
    pub fn prod_tag(&self, id: ProductionId) -> Option<ProdTag> {
        self.prod_tags.get(id.0).copied().flatten()
    }

    pub fn has_prod_tag(&self, id: ProductionId, tag: ProdTag) -> bool {
        self.prod_tag(id) == Some(tag)
    }
}

fn add_tagged_prod<I>(
    g: &mut GrammarBuilder,
    prod_tags: &mut Vec<Option<ProdTag>>,
    lhs: NonTerminalId,
    rhs: I,
    tag: ProdTag,
) -> ProductionId
where
    I: IntoIterator<Item = Symbol>,
{
    let id = g.add_production(lhs, rhs);
    if prod_tags.len() <= id.0 {
        prod_tags.resize(id.0 + 1, None);
    }
    prod_tags[id.0] = Some(tag);
    id
}

pub fn generate_my_grammar_context() -> Option<GrammarContext> {
    match build_my_grammar_context() {
        Ok(context) => Some(context),
        Err(e) => {
            println!("Error building grammar: {:?}", e);
            None
        }
    }
}

#[allow(dead_code)]
pub fn generate_my_grammar() -> Option<Grammar> {
    generate_my_grammar_context().map(|context| context.grammar)
}

fn build_my_grammar_context() -> Result<GrammarContext, GrammarBuilderErr> {
    let mut g = GrammarBuilder::new();
    let mut prod_tags = Vec::new();

    // 关键字
    let i8_ = g.add_terminal("i8");
    let i16_ = g.add_terminal("i16");
    let i32_ = g.add_terminal("i32");
    let i64_ = g.add_terminal("i64");
    let u8_ = g.add_terminal("u8");
    let u16_ = g.add_terminal("u16");
    let u32_ = g.add_terminal("u32");
    let u64_ = g.add_terminal("u64");
    let usize_ = g.add_terminal("usize");
    let isize_ = g.add_terminal("isize");
    let bool_ = g.add_terminal("bool");
    let true_ = g.add_terminal("true");
    let false_ = g.add_terminal("false");
    let let_ = g.add_terminal("let");
    let if_ = g.add_terminal("if");
    let else_ = g.add_terminal("else");
    let while_ = g.add_terminal("while");
    let return_ = g.add_terminal("return");
    let mut_ = g.add_terminal("mut");
    let fn_ = g.add_terminal("fn");
    let for_ = g.add_terminal("for");
    let in_ = g.add_terminal("in");
    let loop_ = g.add_terminal("loop");
    let break_ = g.add_terminal("break");
    let continue_ = g.add_terminal("continue");
    let extern_ = g.add_terminal("extern");
    let str_ = g.add_terminal("str");
    let struct_ = g.add_terminal("struct");

    // 标识符
    let ident = g.add_terminal("id");

    // 数值字面量
    let literal_i32 = g.add_terminal("literal_i32");
    let literal_string = g.add_terminal("literal_string");

    // 赋值号
    let assignment = g.add_terminal("=");

    // 算符
    let plus = g.add_terminal("+");
    let minus = g.add_terminal("-");
    let star = g.add_terminal("*");
    let slash = g.add_terminal("/");
    let eqeq = g.add_terminal("==");
    let gt = g.add_terminal(">");
    let ge = g.add_terminal(">=");
    let lt = g.add_terminal("<");
    let le = g.add_terminal("<=");
    let ne = g.add_terminal("!=");
    let amp = g.add_terminal("&");

    // 界符
    let l_paren = g.add_terminal("(");
    let r_paren = g.add_terminal(")");
    let l_brace = g.add_terminal("[");
    let r_brace = g.add_terminal("]");
    let l_bracket = g.add_terminal("{");
    let r_bracket = g.add_terminal("}");

    // 分隔符
    let comma = g.add_terminal(",");
    let colon = g.add_terminal(":");
    let semicolon = g.add_terminal(";");

    // 特殊符号
    let arrow = g.add_terminal("->");
    let dot = g.add_terminal(".");
    let dotdot = g.add_terminal("..");
    let ellipsis = g.add_terminal("...");

    // 文法
    /*
    Part0
    - 0.1 变量属性
        - ＜变量属性>->mut
    - 0.2 类型
        - ＜类型>-i32
    - 0.3 左值
        - ＜左值>-><可取引用>
        - ＜可取引用>-><可取元素>
        - ＜可取元素>-><ID>
     */
    let var_attr = g.add_non_terminal("var_attr");
    let ty = g.add_non_terminal("ty");
    let l_val = g.add_non_terminal("l_val");
    let addr = g.add_non_terminal("addr");
    let addr_elem = g.add_non_terminal("addr_elem");
    let addr_elem_ident_tail = g.add_non_terminal("addr_elem_ident_tail");

    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        var_attr,
        rhs!(mut_),
        ProdTag::VarAttrMut,
    );
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(i8_), ProdTag::TyI8);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(i16_), ProdTag::TyI16);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(i32_), ProdTag::TyI32);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(i64_), ProdTag::TyI64);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(u8_), ProdTag::TyU8);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(u16_), ProdTag::TyU16);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(u32_), ProdTag::TyU32);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(u64_), ProdTag::TyU64);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(usize_), ProdTag::TyUsize);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(isize_), ProdTag::TyIsize);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(bool_), ProdTag::TyBool);
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(str_), ProdTag::TyStr);
    add_tagged_prod(&mut g, &mut prod_tags, l_val, rhs!(addr), ProdTag::LValAddr);
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr,
        rhs!(addr_elem),
        ProdTag::AddrAddrElem,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(ident, addr_elem_ident_tail),
        ProdTag::AddrElemIdent,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem_ident_tail,
        rhs!(),
        ProdTag::AddrElemIdentTailEmpty,
    );

    /*
       Part1
       1.1 基础程序
       Program -> <声明串＞
       ＜声明串>->空|＜声明><声明串>
       ＜声明>->＜函数声明＞
       ＜函数声明>->＜函数头声明><语句块＞
       ＜函数头声明>->fn <ID>'('<形参列表>')'
       ＜形参列表＞-＞空
       ＜语句块> '{' ＜语句串> '}'
       ＜语句串＞-＞空
    */
    let program = g.add_non_terminal("program");
    let decl_list = g.add_non_terminal("decl_list");
    let decl = g.add_non_terminal("decl");
    let fn_decl = g.add_non_terminal("fn_decl");
    let extern_fn_decl = g.add_non_terminal("extern_fn_decl");
    let fn_sig = g.add_non_terminal("fn_sig");
    let param_list = g.add_non_terminal("param_list");
    let extern_param_list = g.add_non_terminal("extern_param_list");
    let block = g.add_non_terminal("block");
    let stmt_list = g.add_non_terminal("stmt_list");

    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        program,
        rhs!(decl_list),
        ProdTag::ProgramDeclList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        decl_list,
        rhs!(),
        ProdTag::DeclListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        decl_list,
        rhs!(decl, decl_list),
        ProdTag::DeclListDeclDeclList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        decl,
        rhs!(fn_decl),
        ProdTag::DeclFnDecl,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        decl,
        rhs!(extern_fn_decl),
        ProdTag::DeclExternFnDecl,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        fn_decl,
        rhs!(fn_sig, block),
        ProdTag::FnDeclSigBlock,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        fn_sig,
        rhs!(fn_, ident, l_paren, param_list, r_paren),
        ProdTag::FnSigNoRet,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        extern_fn_decl,
        rhs!(
            extern_,
            fn_,
            ident,
            l_paren,
            extern_param_list,
            r_paren,
            semicolon
        ),
        ProdTag::ExternFnDeclNoRet,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        extern_fn_decl,
        rhs!(
            extern_,
            fn_,
            ident,
            l_paren,
            extern_param_list,
            r_paren,
            arrow,
            ty,
            semicolon
        ),
        ProdTag::ExternFnDeclRetTy,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        param_list,
        rhs!(),
        ProdTag::ParamListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        extern_param_list,
        rhs!(),
        ProdTag::ExternParamListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        block,
        rhs!(l_bracket, stmt_list, r_bracket),
        ProdTag::BlockStmtList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt_list,
        rhs!(),
        ProdTag::StmtListEmpty,
    );

    /*
       1.2 语句（拓展1.1）
       <语句串＞->＜语句＞＜语句串＞
       <语句>-> ';'
    */
    let stmt = g.add_non_terminal("stmt");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt_list,
        rhs!(stmt, stmt_list),
        ProdTag::StmtListStmtStmtList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(semicolon),
        ProdTag::StmtEmpty,
    );

    /*
       1.3 返回语句（拓展1.2）
       ＜语句>-><返回语句＞
       <返回语句>-> return ';'
    */
    let return_stmt = g.add_non_terminal("return_stmt");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(return_stmt),
        ProdTag::StmtReturn,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        return_stmt,
        rhs!(return_, semicolon),
        ProdTag::ReturnStmtEmpty,
    );

    /*
       1.4 函数输入（依赖0.1、0.2，拓展1.1）
       <形参列表>-> <形参> | <形参> ',' <形参列表>
       <形参> -> <变量属性> <ID> ':' <类型>
    */
    let param = g.add_non_terminal("param");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        param_list,
        rhs!(param),
        ProdTag::ParamListParam,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        param_list,
        rhs!(param, comma, param_list),
        ProdTag::ParamListParamList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        param,
        rhs!(var_attr, ident, colon, ty),
        ProdTag::ParamVarAttrIdentTy,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        extern_param_list,
        rhs!(param),
        ProdTag::ExternParamListParam,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        extern_param_list,
        rhs!(param, comma, extern_param_list),
        ProdTag::ExternParamListParamList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        extern_param_list,
        rhs!(ellipsis),
        ProdTag::ExternParamListVariadic,
    );

    /*
       1.5 函数输出（依赖0.2、3.1，拓展1.1、1.3）
       <函数头声明> -> fn <ID> '(' <形参列表> ')' '->' <类型>
       <返回语句> -> return <表达式> ';'
    */
    let expr = g.add_non_terminal("expr");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        fn_sig,
        rhs!(fn_, ident, l_paren, param_list, r_paren, arrow, ty),
        ProdTag::FnSigRetTy,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        return_stmt,
        rhs!(return_, expr, semicolon),
        ProdTag::ReturnStmtExpr,
    );

    /*
       Part2
       2.0 变量声明（依赖0.1、0.2）
       <变量声明> -> <变量属性> <ID>
       <变量声明> -> <变量属性> <ID> ':' <类型>
    */
    let var_decl = g.add_non_terminal("var_decl");
    let let_decl = g.add_non_terminal("let_decl");
    let pat = g.add_non_terminal("pat");
    let pat_ident_tail = g.add_non_terminal("pat_ident_tail");
    let pat_tuple_inner = g.add_non_terminal("pat_tuple_inner");
    let pat_list = g.add_non_terminal("pat_list");
    let struct_pat_field_list = g.add_non_terminal("struct_pat_field_list");
    let struct_pat_field = g.add_non_terminal("struct_pat_field");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        var_decl,
        rhs!(var_attr, ident),
        ProdTag::VarDeclNoTy,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        var_decl,
        rhs!(var_attr, ident, colon, ty),
        ProdTag::VarDeclWithTy,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        let_decl,
        rhs!(pat),
        ProdTag::LetDeclNoTy,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        let_decl,
        rhs!(pat, colon, ty),
        ProdTag::LetDeclWithTy,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat,
        rhs!(ident, pat_ident_tail),
        ProdTag::PatIdent,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat_ident_tail,
        rhs!(),
        ProdTag::PatIdentTailEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat,
        rhs!(mut_, ident),
        ProdTag::PatMutIdent,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat,
        rhs!(l_paren, pat_tuple_inner, r_paren),
        ProdTag::PatTuple,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat_tuple_inner,
        rhs!(),
        ProdTag::PatTupleInnerEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat_tuple_inner,
        rhs!(pat, comma, pat_list),
        ProdTag::PatTupleInnerPat,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat_list,
        rhs!(),
        ProdTag::PatListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat_list,
        rhs!(pat),
        ProdTag::PatListPat,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat_list,
        rhs!(pat, comma, pat_list),
        ProdTag::PatListPatList,
    );

    /*
       2.1 变量声明语句（依赖2.0，拓展1.2）
       <语句> -> <变量声明语句>
       <变量声明语句> -> let <变量声明> ';'
    */
    let var_decl_stmt = g.add_non_terminal("var_decl_stmt");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(var_decl_stmt),
        ProdTag::StmtVarDecl,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        var_decl_stmt,
        rhs!(let_, let_decl, semicolon),
        ProdTag::VarDeclStmt,
    );

    /*
       2.2 赋值语句（依赖0.3、3.1，拓展1.2）
       <语句>-> <赋值语句>
       <赋值语句> -> <左值> '=' <表达式> ';'
    */
    let assign_stmt = g.add_non_terminal("assign_stmt");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(assign_stmt),
        ProdTag::StmtAssign,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        assign_stmt,
        rhs!(l_val, assignment, expr, semicolon),
        ProdTag::AssignStmt,
    );

    /*
       2.3 变量声明赋值语句（依赖2.0、3.1、拓展1.2）
       <语句>-> <变量声明赋值语句>
       <变量声明赋值语句> -> let <变量声明> '=' <表达式> ';'
    */
    let var_init_stmt = g.add_non_terminal("var_init_stmt");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(var_init_stmt),
        ProdTag::StmtVarInit,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        var_init_stmt,
        rhs!(let_, let_decl, assignment, expr, semicolon),
        ProdTag::VarInitStmt,
    );

    /*
       Part3
       3.1 基本表达式（依赖0.3、拓展1.2）
       <语句> -> <表达式> ';'
       <表达式> -> <加减表达式>
       <加减表达式> -> <项>
       <项> -> <因子>
       <因子> -> <NUM> | <左值> | '(' <表达式> ')'
    */
    let add_expr = g.add_non_terminal("add_expr");
    let term = g.add_non_terminal("term");
    let factor = g.add_non_terminal("factor");
    let num = g.add_non_terminal("num");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(expr, semicolon),
        ProdTag::StmtExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        expr,
        rhs!(add_expr),
        ProdTag::ExprAdd,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        add_expr,
        rhs!(term),
        ProdTag::AddExprTerm,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        term,
        rhs!(factor),
        ProdTag::TermFactor,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        factor,
        rhs!(num),
        ProdTag::FactorNum,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        factor,
        rhs!(true_),
        ProdTag::FactorTrue,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        factor,
        rhs!(false_),
        ProdTag::FactorFalse,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        factor,
        rhs!(literal_string),
        ProdTag::FactorString,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        num,
        rhs!(literal_i32),
        ProdTag::NumLiteralI32,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        factor,
        rhs!(l_val),
        ProdTag::FactorLVal,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        factor,
        rhs!(l_paren, expr, r_paren),
        ProdTag::FactorGroupedExpr,
    );

    /*
       3.2 增加比较运算（依赖3.1、拓展3.1）
       <表达式> -> <表达式> <比较运算符> <加减表达式>
       <比较运算符> -> '<' | '<=' | '>' | '>=' | '==' | '!=‘
    */
    let cmp_op = g.add_non_terminal("cmp_op");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        expr,
        rhs!(expr, cmp_op, add_expr),
        ProdTag::ExprCmp,
    );
    add_tagged_prod(&mut g, &mut prod_tags, cmp_op, rhs!(lt), ProdTag::CmpOpLt);
    add_tagged_prod(&mut g, &mut prod_tags, cmp_op, rhs!(le), ProdTag::CmpOpLe);
    add_tagged_prod(&mut g, &mut prod_tags, cmp_op, rhs!(gt), ProdTag::CmpOpGt);
    add_tagged_prod(&mut g, &mut prod_tags, cmp_op, rhs!(ge), ProdTag::CmpOpGe);
    add_tagged_prod(&mut g, &mut prod_tags, cmp_op, rhs!(eqeq), ProdTag::CmpOpEq);
    add_tagged_prod(&mut g, &mut prod_tags, cmp_op, rhs!(ne), ProdTag::CmpOpNe);

    /*
       3.3 增加加减运算（依赖3.1、拓展3.1）
       <加减表达式> -> <加减表达式> <加减运算符> <项>
       <加减运算符> -> '+' | '-
    */
    let add_op = g.add_non_terminal("add_op");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        add_expr,
        rhs!(add_expr, add_op, term),
        ProdTag::AddExprBinary,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        add_op,
        rhs!(plus),
        ProdTag::AddOpPlus,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        add_op,
        rhs!(minus),
        ProdTag::AddOpMinus,
    );

    /*
       3.4 增加乘除运算（依赖3.1、拓展3.1）
       <项> -> <项> <乘除运算符> <因子>
       <乘除运算符> -> '*' | '/’
    */
    let mul_op = g.add_non_terminal("mul_op");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        term,
        rhs!(term, mul_op, factor),
        ProdTag::TermBinary,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        mul_op,
        rhs!(star),
        ProdTag::MulOpStar,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        mul_op,
        rhs!(slash),
        ProdTag::MulOpSlash,
    );

    /*
       3.5 增加函数调用（依赖3.1、拓展3.1）
       <因子> -> <ID> '(' <实参列表> ')'
       <实参列表>-> 空 | <表达式> | <表达式> ',' <实参列表>
    */
    let arg_list = g.add_non_terminal("arg_list");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        factor,
        rhs!(ident, l_paren, arg_list, r_paren),
        ProdTag::FactorCall,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        arg_list,
        rhs!(),
        ProdTag::ArgListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        arg_list,
        rhs!(expr),
        ProdTag::ArgListExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        arg_list,
        rhs!(expr, comma, arg_list),
        ProdTag::ArgListExprList,
    );

    /*
       3.6 条件表达式（用于 if/while 条件后面紧跟语句块的位置）
       这里排除裸结构体字面量，避免 `if Foo { ... }` 和 `Foo { ... }`
       之间的经典二义性；需要结构体字面量参与条件表达式时可写成 `(Foo { ... })`。
    */
    let cond_expr = g.add_non_terminal("cond_expr");
    let cond_add_expr = g.add_non_terminal("cond_add_expr");
    let cond_term = g.add_non_terminal("cond_term");
    let cond_factor = g.add_non_terminal("cond_factor");
    let cond_l_val = g.add_non_terminal("cond_l_val");
    let cond_addr = g.add_non_terminal("cond_addr");
    let cond_addr_elem = g.add_non_terminal("cond_addr_elem");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_expr,
        rhs!(cond_add_expr),
        ProdTag::ExprAdd,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_expr,
        rhs!(cond_expr, cmp_op, cond_add_expr),
        ProdTag::ExprCmp,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_add_expr,
        rhs!(cond_term),
        ProdTag::AddExprTerm,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_add_expr,
        rhs!(cond_add_expr, add_op, cond_term),
        ProdTag::AddExprBinary,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_term,
        rhs!(cond_factor),
        ProdTag::TermFactor,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_term,
        rhs!(cond_term, mul_op, cond_factor),
        ProdTag::TermBinary,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_factor,
        rhs!(num),
        ProdTag::FactorNum,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_factor,
        rhs!(true_),
        ProdTag::FactorTrue,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_factor,
        rhs!(false_),
        ProdTag::FactorFalse,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_factor,
        rhs!(literal_string),
        ProdTag::FactorString,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_factor,
        rhs!(cond_l_val),
        ProdTag::FactorLVal,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_factor,
        rhs!(l_paren, expr, r_paren),
        ProdTag::FactorGroupedExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_factor,
        rhs!(ident, l_paren, arg_list, r_paren),
        ProdTag::FactorCall,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_l_val,
        rhs!(cond_addr),
        ProdTag::LValAddr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_l_val,
        rhs!(star, cond_l_val),
        ProdTag::LValDeref,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_addr,
        rhs!(cond_addr_elem),
        ProdTag::AddrAddrElem,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_addr,
        rhs!(amp, cond_addr),
        ProdTag::AddrRef,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_addr,
        rhs!(amp, mut_, cond_addr),
        ProdTag::AddrRefMut,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_addr_elem,
        rhs!(ident),
        ProdTag::AddrElemIdent,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_addr_elem,
        rhs!(cond_addr_elem, l_brace, expr, r_brace),
        ProdTag::AddrElemIndex,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_addr_elem,
        rhs!(cond_addr_elem, dot, num),
        ProdTag::AddrElemField,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_addr_elem,
        rhs!(cond_addr_elem, dot, ident),
        ProdTag::AddrElemNamedField,
    );

    /*
       Part4
       4.1 选择结构（依赖1.1、3.1，拓展1.2）
       <语句> -> <if语句>
       <if语句> -> if <表达式> <语句块> <else部分>
       <else部分> -> 空
    */
    let if_stmt = g.add_non_terminal("if_stmt");
    let else_part = g.add_non_terminal("else_part");
    add_tagged_prod(&mut g, &mut prod_tags, stmt, rhs!(if_stmt), ProdTag::StmtIf);
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        if_stmt,
        rhs!(if_, cond_expr, block, else_part),
        ProdTag::IfStmt,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        else_part,
        rhs!(),
        ProdTag::ElsePartEmpty,
    );

    /*
       4.2 增加else（依赖1.1，拓展4.1）
       <else部分> -> else <语句块>
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        else_part,
        rhs!(else_, block),
        ProdTag::ElsePartBlock,
    );

    /*
       4.3 增加else if（依赖1.1，拓展4.1）
       <else部分> -> else if <表达式> <语句块> <else部分>
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        else_part,
        rhs!(else_, if_, cond_expr, block, else_part),
        ProdTag::ElsePartIf,
    );

    /*
       Part5
       5.0 循环语句（依赖5.1 or 5.2 or 5.3，拓展1.2）
       <语句> -> <循环语句>
    */
    let loop_stmt = g.add_non_terminal("loop_stmt");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(loop_stmt),
        ProdTag::StmtLoop,
    );

    /*
       5.1 while循环（依赖1.1、3.1，拓展5.0）
       <循环语句> -> <while语句>
       <while语句> -> while <表达式> <语句块>
    */
    let while_stmt = g.add_non_terminal("while_stmt");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        loop_stmt,
        rhs!(while_stmt),
        ProdTag::LoopStmtWhile,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        while_stmt,
        rhs!(while_, cond_expr, block),
        ProdTag::WhileStmt,
    );

    /*
       5.2 for循环（依赖1.1、2.0、3.1，拓展5.0）
       <循环语句> -> <for语句>
       <for语句>-> for <变量声明> in <可迭代结构> <语句块>
       <可迭代结构> -> <表达式> '..' <表达式>
    */
    let for_stmt = g.add_non_terminal("for_stmt");
    let range_expr = g.add_non_terminal("range_expr");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        loop_stmt,
        rhs!(for_stmt),
        ProdTag::LoopStmtFor,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        for_stmt,
        rhs!(for_, var_decl, in_, range_expr, block),
        ProdTag::ForStmt,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        for_stmt,
        rhs!(for_, var_decl, in_, cond_expr, block),
        ProdTag::ForStmtIter,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        range_expr,
        rhs!(cond_expr, dotdot, cond_expr),
        ProdTag::RangeExpr,
    );

    /*
       5.3 loop循环（依赖1.1，拓展5.0）
       <循环语句> -> <loop语句> # 修改：仅保留语句形式
       <loop语句> -> loop <语句块> # 修改：与loop表达式分离
    */
    let infinite_loop_stmt = g.add_non_terminal("infinite_loop_stmt");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        loop_stmt,
        rhs!(infinite_loop_stmt),
        ProdTag::LoopStmtInfinite,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        infinite_loop_stmt,
        rhs!(loop_, block),
        ProdTag::InfiniteLoopStmt,
    );

    /*
       5.4 增加break和continue（拓展1.2）
       <语句> -> break ';' | continue ';'
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(break_, semicolon),
        ProdTag::StmtBreak,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(continue_, semicolon),
        ProdTag::StmtContinue,
    );

    /*
       Part6
       6.1 变量不可变属性（拓展0.1）
       <变量属性> -> 空
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        var_attr,
        rhs!(),
        ProdTag::VarAttrEmpty,
    );

    /*
       6.2 不可变引用（依赖0.2、0.3，拓展0.2、0.3）
       <类型> -> '&' <类型>
       <可取引用> -> '&' <可取引用>
    */
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(amp, ty), ProdTag::TyRef);
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr,
        rhs!(amp, addr),
        ProdTag::AddrRef,
    );

    /*
       6.3 可变引用（依赖0.2、0.3，拓展0.2、0.3）
       <类型> -> '&' mut <类型>
       <可取引用> -> '&' mut <可取引用>
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        ty,
        rhs!(amp, mut_, ty),
        ProdTag::TyRefMut,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr,
        rhs!(amp, mut_, addr),
        ProdTag::AddrRefMut,
    );

    /*
       6.4 借用（依赖0.3、拓展0.3）
       <左值> -> '*' <左值>
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        l_val,
        rhs!(star, l_val),
        ProdTag::LValDeref,
    );

    /*
       Part7
       7.0 函数表达式块（依赖1.1、3.1，拓展3.1）
       <函数表达式语句块>->'{' <函数表达式语句串> '}'
       <函数表达式语句串>-> <表达式> | <语句> <函数表达式语句串>
    */
    let block_expr = g.add_non_terminal("block_expr");
    let block_expr_body = g.add_non_terminal("block_expr_body");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        block_expr,
        rhs!(l_bracket, block_expr_body, r_bracket),
        ProdTag::BlockExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        block_expr_body,
        rhs!(expr),
        ProdTag::BlockExprBodyExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        block_expr_body,
        rhs!(stmt, block_expr_body),
        ProdTag::BlockExprBodyStmt,
    );

    /*
       7.1 函数表达式块作为表达式（依赖7.0、拓展0.3）
       <可取元素> -> <函数表达式语句块>
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(block_expr),
        ProdTag::AddrElemBlockExpr,
    );

    /*
       7.2 函数表达式块作为函数体（依赖1.1、7.0 ，拓展1.1）
       <函数声明> -> <函数头声明> <函数表达式语句块>
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        fn_decl,
        rhs!(fn_sig, block_expr),
        ProdTag::FnDeclSigBlockExpr,
    );

    /*
       7.3 选择表达式（依赖3.1、7.0，拓展0.3）
       <可取元素> -> <选择表达式>
       <选择表达式>-> if <表达式> <函数表达式语句块> else <函数表达式语句块>
    */
    let branch_expr = g.add_non_terminal("branch_expr");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(branch_expr),
        ProdTag::AddrElemBranchExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        branch_expr,
        rhs!(if_, cond_expr, block_expr, else_, block_expr),
        ProdTag::BranchExpr,
    );

    /*
       7.4 循环表达式（依赖3.1、7.0，拓展0.3） # 修改
       <可取元素> -> <loop表达式> # 修改
       <loop表达式> -> loop <函数表达式语句块> # 修改
       <语句> -> break <表达式> ';' # 修改：为loop表达式提供值
    */
    let loop_expr = g.add_non_terminal("loop_expr");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(loop_expr),
        ProdTag::AddrElemLoopExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        loop_expr,
        rhs!(loop_, block_expr),
        ProdTag::LoopExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        stmt,
        rhs!(break_, expr, semicolon),
        ProdTag::StmtBreakExpr,
    );

    /*
       Part8
       8.1 数组类型（依赖0.2，拓展0.2）
       <类型> -> '[' <类型> ';' <NUM>’]‘
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        ty,
        rhs!(l_brace, ty, semicolon, num, r_brace),
        ProdTag::TyArray,
    );

    /*
       8.2 数组表达式（依赖3.1，拓展0.3、5.2）
       <可取元素> -> '[' <数组元素列表> ']'
       <数组元素列表>-> 空 | <表达式> | <表达式> ',' <数组元素列表>
       <可迭代结构> -> <表达式>
    */
    let array_elem_list = g.add_non_terminal("array_elem_list");
    let iter_expr = g.add_non_terminal("iter_expr");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(l_brace, array_elem_list, r_brace),
        ProdTag::AddrElemArray,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        cond_addr_elem,
        rhs!(l_brace, array_elem_list, r_brace),
        ProdTag::AddrElemArray,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        array_elem_list,
        rhs!(),
        ProdTag::ArrayElemListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        array_elem_list,
        rhs!(expr),
        ProdTag::ArrayElemListExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        array_elem_list,
        rhs!(expr, comma, array_elem_list),
        ProdTag::ArrayElemListExprList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        iter_expr,
        rhs!(expr),
        ProdTag::IterExpr,
    );

    /*
       8.3 数组元素（依赖0.3、3.1，拓展0.3）
       <可取元素> -> <可取元素> '[' <表达式> ']'
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(addr_elem, l_brace, expr, r_brace),
        ProdTag::AddrElemIndex,
    );

    /*
       Part9
       9.1 元组类型（依赖0.2，拓展0.2）
       <类型> -> '(' <元组类型内部> ')'
       <元组类型内部> -> 空 | <类型> ',' <类型列表>
       <类型列表> ->空 | <类型> | <类型> ',' <类型列表>
    */
    let tuple_ty_inner = g.add_non_terminal("tuple_ty_inner");
    let ty_list = g.add_non_terminal("ty_list");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        ty,
        rhs!(l_paren, tuple_ty_inner, r_paren),
        ProdTag::TyTuple,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        tuple_ty_inner,
        rhs!(),
        ProdTag::TupleTyInnerEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        tuple_ty_inner,
        rhs!(ty, comma, ty_list),
        ProdTag::TupleTyInnerTy,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        ty_list,
        rhs!(),
        ProdTag::TyListEmpty,
    );
    add_tagged_prod(&mut g, &mut prod_tags, ty_list, rhs!(ty), ProdTag::TyListTy);
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        ty_list,
        rhs!(ty, comma, ty_list),
        ProdTag::TyListTyList,
    );

    /*
       9.2 元组表达式（依赖3.1、拓展0.3）
       <可取元素> -> '(' <元组赋值内部> ')'
       <元组赋值内部> -> 空 | <表达式> ',' <元组元素列表>
       <元组元素列表>->空 | <表达式> | <表达式> ',' <元组元素列表>
    */
    let tuple_expr_inner = g.add_non_terminal("tuple_expr_inner");
    let tuple_elem_list = g.add_non_terminal("tuple_elem_list");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(l_paren, tuple_expr_inner, r_paren),
        ProdTag::AddrElemTuple,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        tuple_expr_inner,
        rhs!(),
        ProdTag::TupleExprInnerEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        tuple_expr_inner,
        rhs!(expr, comma, tuple_elem_list),
        ProdTag::TupleExprInnerExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        tuple_elem_list,
        rhs!(),
        ProdTag::TupleElemListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        tuple_elem_list,
        rhs!(expr),
        ProdTag::TupleElemListExpr,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        tuple_elem_list,
        rhs!(expr, comma, tuple_elem_list),
        ProdTag::TupleElemListExprList,
    );

    /*
       9.3 元组元素（依赖0.3，拓展0.3）
       <可取元素> -><可取元素> '.' <NUM>
    */
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(addr_elem, dot, num),
        ProdTag::AddrElemField,
    );

    /*
       Part10
       10.1 命名结构体（拓展顶层声明与类型）
       <声明> -> <结构体声明>
       <结构体声明> -> struct <ID> '{' <结构体字段列表> '}'
       <结构体字段列表> -> 空 | <结构体字段> | <结构体字段> ',' <结构体字段列表>
       <结构体字段> -> <ID> ':' <类型>
       <类型> -> <ID>
    */
    let struct_decl = g.add_non_terminal("struct_decl");
    let struct_field_list = g.add_non_terminal("struct_field_list");
    let struct_field = g.add_non_terminal("struct_field");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        decl,
        rhs!(struct_decl),
        ProdTag::DeclStructDecl,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_decl,
        rhs!(struct_, ident, l_bracket, struct_field_list, r_bracket),
        ProdTag::StructDeclNamed,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_field_list,
        rhs!(),
        ProdTag::StructFieldListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_field_list,
        rhs!(struct_field),
        ProdTag::StructFieldListField,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_field_list,
        rhs!(struct_field, comma, struct_field_list),
        ProdTag::StructFieldListFieldList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_field,
        rhs!(ident, colon, ty),
        ProdTag::StructFieldNamed,
    );
    add_tagged_prod(&mut g, &mut prod_tags, ty, rhs!(ident), ProdTag::TyAdt);

    /*
       10.2 结构体字面量与命名字段访问
       <因子> -> <ID> '{' <结构体字面量字段列表> '}'
       <结构体字面量字段列表> -> 空 | <结构体字面量字段> | <结构体字面量字段> ',' <结构体字面量字段列表>
       <结构体字面量字段> -> <ID> ':' <表达式>
       <可取元素> -> <可取元素> '.' <ID>
    */
    let struct_lit_field_list = g.add_non_terminal("struct_lit_field_list");
    let struct_lit_field = g.add_non_terminal("struct_lit_field");
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem_ident_tail,
        rhs!(l_bracket, struct_lit_field_list, r_bracket),
        ProdTag::AddrElemIdentTailStructLit,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_lit_field_list,
        rhs!(),
        ProdTag::StructLitFieldListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_lit_field_list,
        rhs!(struct_lit_field),
        ProdTag::StructLitFieldListField,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_lit_field_list,
        rhs!(struct_lit_field, comma, struct_lit_field_list),
        ProdTag::StructLitFieldListFieldList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_lit_field,
        rhs!(ident, colon, expr),
        ProdTag::StructLitFieldNamed,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        pat_ident_tail,
        rhs!(l_bracket, struct_pat_field_list, r_bracket),
        ProdTag::PatIdentTailStruct,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_pat_field_list,
        rhs!(),
        ProdTag::StructPatFieldListEmpty,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_pat_field_list,
        rhs!(struct_pat_field),
        ProdTag::StructPatFieldListField,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_pat_field_list,
        rhs!(struct_pat_field, comma, struct_pat_field_list),
        ProdTag::StructPatFieldListFieldList,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        struct_pat_field,
        rhs!(ident),
        ProdTag::StructPatFieldNamed,
    );
    add_tagged_prod(
        &mut g,
        &mut prod_tags,
        addr_elem,
        rhs!(addr_elem, dot, ident),
        ProdTag::AddrElemNamedField,
    );

    g.set_start(program);
    let eof = g.add_terminal("eof");
    g.set_eof(eof);
    let terminals = Terminals {
        i8_,
        i16_,
        i32_,
        i64_,
        u8_,
        u16_,
        u32_,
        u64_,
        usize_,
        isize_,
        bool_,
        true_,
        false_,
        let_,
        if_,
        else_,
        while_,
        return_,
        mut_,
        fn_,
        for_,
        in_,
        loop_,
        break_,
        continue_,
        extern_,
        str_,
        struct_,
        ident,
        literal_i32,
        literal_string,
        assignment,
        plus,
        minus,
        star,
        slash,
        eqeq,
        gt,
        ge,
        lt,
        le,
        ne,
        amp,
        l_paren,
        r_paren,
        l_brace,
        r_brace,
        l_bracket,
        r_bracket,
        comma,
        colon,
        semicolon,
        arrow,
        dot,
        dotdot,
        ellipsis,
        eof,
    };
    let grammar = g.build()?;
    Ok(GrammarContext {
        grammar,
        terminals,
        prod_tags,
    })
}
