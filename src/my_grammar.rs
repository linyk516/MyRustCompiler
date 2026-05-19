use crate::parser::grammar::{Grammar, GrammarBuilder, GrammarBuilderErr};
use crate::parser::symbol::TerminalId;
use serde::{Deserialize, Serialize};
/// 方便转换为Symbol，避免into调用过于冗长

macro_rules! rhs {
    ($($x:expr),* $(,)?) => {
        [$( $x.into() ),*]
    };
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Terminals {
    pub i32_: TerminalId,
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
    pub ident: TerminalId,
    pub literal_i32: TerminalId,
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
    pub eof: TerminalId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarContext {
    pub grammar: Grammar,
    pub terminals: Terminals,
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

pub fn generate_my_grammar() -> Option<Grammar> {
    generate_my_grammar_context().map(|context| context.grammar)
}

fn build_my_grammar_context() -> Result<GrammarContext, GrammarBuilderErr> {
    let mut g = GrammarBuilder::new();

    // 关键字
    let i32_ = g.add_terminal("i32");
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

    // 标识符
    let ident = g.add_terminal("id");

    // 数值字面量
    let literal_i32 = g.add_terminal("literal_i32");

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

    g.add_production(var_attr, rhs!(mut_));
    g.add_production(ty, rhs!(i32_));
    g.add_production(l_val, rhs!(addr));
    g.add_production(addr, rhs!(addr_elem));
    g.add_production(addr_elem, rhs!(ident));

    /*
       Part1
       1.1 基础程序
       Program -> <声明串＞＜声明串>->空|＜声明><声明串>
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
    let fn_sig = g.add_non_terminal("fn_sig");
    let param_list = g.add_non_terminal("param_list");
    let block = g.add_non_terminal("block");
    let stmt_list = g.add_non_terminal("stmt_list");

    g.add_production(program, rhs!(decl_list));
    g.add_production(decl_list, rhs!());
    g.add_production(decl_list, rhs!(decl, decl_list));
    g.add_production(decl, rhs!(fn_decl));
    g.add_production(fn_decl, rhs!(fn_sig, block));
    g.add_production(fn_sig, rhs!(fn_, ident, l_paren, param_list, r_paren));
    g.add_production(param_list, rhs!());
    g.add_production(block, rhs!(l_bracket, stmt_list, r_bracket));
    g.add_production(stmt_list, rhs!());

    /*
       1.2 语句（拓展1.1）
       <语句串＞->＜语句＞＜语句串＞
       <语句>-> ';'
    */
    let stmt = g.add_non_terminal("stmt");
    g.add_production(stmt_list, rhs!(stmt, stmt_list));
    g.add_production(stmt, rhs!(semicolon));

    /*
       1.3 返回语句（拓展1.2）
       ＜语句>-><返回语句＞
       <返回语句>-> return ';'
    */
    let return_stmt = g.add_non_terminal("return_stmt");
    g.add_production(stmt, rhs!(return_stmt));
    g.add_production(return_stmt, rhs!(return_, semicolon));

    /*
       1.4 函数输入（依赖0.1、0.2，拓展1.1）
       <形参列表>-> <形参> | <形参> ',' <形参列表>
       <形参> -> <变量属性> <ID> ':' <类型>
    */
    let param = g.add_non_terminal("param");
    g.add_production(param_list, rhs!(param));
    g.add_production(param_list, rhs!(param, comma, param_list));
    g.add_production(param, rhs!(var_attr, ident, colon, ty));

    /*
       1.5 函数输出（依赖0.2、3.1，拓展1.1、1.3）
       <函数头声明> -> fn <ID> '(' <形参列表> ')' '->' <类型>
       <返回语句> -> return <表达式> ';'
    */
    let expr = g.add_non_terminal("expr");
    g.add_production(
        fn_sig,
        rhs!(fn_, ident, l_paren, param_list, r_paren, arrow, ty),
    );
    g.add_production(return_stmt, rhs!(return_, expr, semicolon));

    /*
       Part2
       2.0 变量声明（依赖0.1、0.2）
       <变量声明> -> <变量属性> <ID>
       <变量声明> -> <变量属性> <ID> ':' <类型>
    */
    let var_decl = g.add_non_terminal("var_decl");
    g.add_production(var_decl, rhs!(var_attr, ident));
    g.add_production(var_decl, rhs!(var_attr, ident, colon, ty));

    /*
       2.1 变量声明语句（依赖2.0，拓展1.2）
       <语句> -> <变量声明语句>
       <变量声明语句> -> let <变量声明> ';'
    */
    let var_decl_stmt = g.add_non_terminal("var_decl_stmt");
    g.add_production(stmt, rhs!(var_decl_stmt));
    g.add_production(var_decl_stmt, rhs!(let_, var_decl, semicolon));

    /*
       2.2 赋值语句（依赖0.3、3.1，拓展1.2）
       <语句>-> <赋值语句>
       <赋值语句> -> <左值> '=' <表达式> ';'
    */
    let assign_stmt = g.add_non_terminal("assign_stmt");
    g.add_production(stmt, rhs!(assign_stmt));
    g.add_production(assign_stmt, rhs!(l_val, assignment, expr, semicolon));

    /*
       2.3 变量声明赋值语句（依赖2.0、3.1、拓展1.2）
       <语句>-> <变量声明赋值语句>
       <变量声明赋值语句> -> let <变量声明> '=' <表达式> ';'
    */
    let var_init_stmt = g.add_non_terminal("var_init_stmt");
    g.add_production(stmt, rhs!(var_init_stmt));
    g.add_production(
        var_init_stmt,
        rhs!(let_, var_decl, assignment, expr, semicolon),
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
    g.add_production(stmt, rhs!(expr, semicolon));
    g.add_production(expr, rhs!(add_expr));
    g.add_production(add_expr, rhs!(term));
    g.add_production(term, rhs!(factor));
    g.add_production(factor, rhs!(num));
    g.add_production(num, rhs!(literal_i32));
    g.add_production(factor, rhs!(l_val));
    g.add_production(factor, rhs!(l_paren, expr, r_paren));

    /*
       3.2 增加比较运算（依赖3.1、拓展3.1）
       <表达式> -> <表达式> <比较运算符> <加减表达式>
       <比较运算符> -> '<' | '<=' | '>' | '>=' | '==' | '!=‘
    */
    let cmp_op = g.add_non_terminal("cmp_op");
    g.add_production(expr, rhs!(expr, cmp_op, add_expr));
    g.add_production(cmp_op, rhs!(lt));
    g.add_production(cmp_op, rhs!(le));
    g.add_production(cmp_op, rhs!(gt));
    g.add_production(cmp_op, rhs!(ge));
    g.add_production(cmp_op, rhs!(eqeq));
    g.add_production(cmp_op, rhs!(ne));

    /*
       3.3 增加加减运算（依赖3.1、拓展3.1）
       <加减表达式> -> <加减表达式> <加减运算符> <项>
       <加减运算符> -> '+' | '-
    */
    let add_op = g.add_non_terminal("add_op");
    g.add_production(add_expr, rhs!(add_expr, add_op, term));
    g.add_production(add_op, rhs!(plus));
    g.add_production(add_op, rhs!(minus));

    /*
       3.4 增加乘除运算（依赖3.1、拓展3.1）
       <项> -> <项> <乘除运算符> <因子>
       <乘除运算符> -> '*' | '/’
    */
    let mul_op = g.add_non_terminal("mul_op");
    g.add_production(term, rhs!(term, mul_op, factor));
    g.add_production(mul_op, rhs!(star));
    g.add_production(mul_op, rhs!(slash));

    /*
       3.5 增加函数调用（依赖3.1、拓展3.1）
       <因子> -> <ID> '(' <实参列表> ')'
       <实参列表>-> 空 | <表达式> | <表达式> ',' <实参列表>
    */
    let arg_list = g.add_non_terminal("arg_list");
    g.add_production(factor, rhs!(ident, l_paren, arg_list, r_paren));
    g.add_production(arg_list, rhs!());
    g.add_production(arg_list, rhs!(expr));
    g.add_production(arg_list, rhs!(expr, comma, arg_list));

    /*
       Part4
       4.1 选择结构（依赖1.1、3.1，拓展1.2）
       <语句> -> <if语句>
       <if语句> -> if <表达式> <语句块> <else部分>
       <else部分> -> 空
    */
    let if_stmt = g.add_non_terminal("if_stmt");
    let else_part = g.add_non_terminal("else_part");
    g.add_production(stmt, rhs!(if_stmt));
    g.add_production(if_stmt, rhs!(if_, expr, block, else_part));
    g.add_production(else_part, rhs!());

    /*
       4.2 增加else（依赖1.1，拓展4.1）
       <else部分> -> else <语句块>
    */
    g.add_production(else_part, rhs!(else_, block));

    /*
       4.3 增加else if（依赖1.1，拓展4.1）
       <else部分> -> else if <表达式> <语句块> <else部分>
    */
    g.add_production(else_part, rhs!(else_, if_, expr, block, else_part));

    /*
       Part5
       5.0 循环语句（依赖5.1 or 5.2 or 5.3，拓展1.2）
       <语句> -> <循环语句>
    */
    let loop_stmt = g.add_non_terminal("loop_stmt");
    g.add_production(stmt, rhs!(loop_stmt));

    /*
       5.1 while循环（依赖1.1、3.1，拓展5.0）
       <循环语句> -> <while语句>
       <while语句> -> while <表达式> <语句块>
    */
    let while_stmt = g.add_non_terminal("while_stmt");
    g.add_production(loop_stmt, rhs!(while_stmt));
    g.add_production(while_stmt, rhs!(while_, expr, block));

    /*
       5.2 for循环（依赖1.1、2.0、3.1，拓展5.0）
       <循环语句> -> <for语句>
       <for语句>-> for <变量声明> in <可迭代结构> <语句块>
       <可迭代结构> -> <表达式> '..' <表达式>
    */
    let for_stmt = g.add_non_terminal("for_stmt");
    let range_expr = g.add_non_terminal("range_expr");
    g.add_production(loop_stmt, rhs!(for_stmt));
    g.add_production(for_stmt, rhs!(for_, var_decl, in_, range_expr, block));
    g.add_production(range_expr, rhs!(expr, dotdot, expr));

    /*
       5.3 loop循环（依赖1.1，拓展5.0）
       <循环语句> -> <loop语句> # 修改：仅保留语句形式
       <loop语句> -> loop <语句块> # 修改：与loop表达式分离
    */
    let infinite_loop_stmt = g.add_non_terminal("infinite_loop_stmt");
    g.add_production(loop_stmt, rhs!(infinite_loop_stmt));
    g.add_production(infinite_loop_stmt, rhs!(loop_, block));

    /*
       5.4 增加break和continue（拓展1.2）
       <语句> -> break ';' | continue ';'
    */
    g.add_production(stmt, rhs!(break_, semicolon));
    g.add_production(stmt, rhs!(continue_, semicolon));

    /*
       Part6
       6.1 变量不可变属性（拓展0.1）
       <变量属性> -> 空
    */
    g.add_production(var_attr, rhs!());

    /*
       6.2 不可变引用（依赖0.2、0.3，拓展0.2、0.3）
       <类型> -> '&' <类型>
       <可取引用> -> '&' <可取引用>
    */
    g.add_production(ty, rhs!(amp, ty));
    g.add_production(addr, rhs!(amp, addr));

    /*
       6.3 可变引用（依赖0.2、0.3，拓展0.2、0.3）
       <类型> -> '&' mut <类型>
       <可取引用> -> '&' mut <可取引用>
    */
    g.add_production(ty, rhs!(amp, mut_, ty));
    g.add_production(addr, rhs!(amp, mut_, addr));

    /*
       6.4 借用（依赖0.3、拓展0.3）
       <左值> -> '*' <左值>
    */
    g.add_production(l_val, rhs!(star, l_val));

    /*
       Part7
       7.0 函数表达式块（依赖1.1、3.1，拓展3.1）
       <函数表达式语句块>->'{' <函数表达式语句串> '}'
       <函数表达式语句串>-> <表达式> | <语句> <函数表达式语句串>
    */
    let block_expr = g.add_non_terminal("block_expr");
    let block_expr_body = g.add_non_terminal("block_expr_body");
    g.add_production(block_expr, rhs!(l_bracket, block_expr_body, r_bracket));
    g.add_production(block_expr_body, rhs!(expr));
    g.add_production(block_expr_body, rhs!(stmt, block_expr_body));

    /*
       7.1 函数表达式块作为表达式（依赖7.0、拓展0.3）
       <可取元素> -> <函数表达式语句块>
    */
    g.add_production(addr_elem, rhs!(block_expr));

    /*
       7.2 函数表达式块作为函数体（依赖1.1、7.0 ，拓展1.1）
       <函数声明> -> <函数头声明> <函数表达式语句块>
    */
    g.add_production(fn_decl, rhs!(fn_sig, block_expr));

    /*
       7.3 选择表达式（依赖3.1、7.0，拓展0.3）
       <可取元素> -> <选择表达式>
       <选择表达式>-> if <表达式> <函数表达式语句块> else <函数表达式语句块>
    */
    let branch_expr = g.add_non_terminal("branch_expr");
    g.add_production(addr_elem, rhs!(branch_expr));
    g.add_production(branch_expr, rhs!(if_, expr, block_expr, else_, block_expr));

    /*
       7.4 循环表达式（依赖3.1、7.0，拓展0.3） # 修改
       <可取元素> -> <loop表达式> # 修改
       <loop表达式> -> loop <函数表达式语句块> # 修改
       <语句> -> break <表达式> ';' # 修改：为loop表达式提供值
    */
    let loop_expr = g.add_non_terminal("loop_expr");
    g.add_production(addr_elem, rhs!(loop_expr));
    g.add_production(loop_expr, rhs!(loop_, block_expr));
    g.add_production(stmt, rhs!(break_, expr, semicolon));

    /*
       Part8
       8.1 数组类型（依赖0.2，拓展0.2）
       <类型> -> '[' <类型> ';' <NUM>’]‘
    */
    g.add_production(ty, rhs!(l_brace, ty, semicolon, num, r_brace));

    /*
       8.2 数组表达式（依赖3.1，拓展0.3、5.2）
       <可取元素> -> '[' <数组元素列表> ']'
       <数组元素列表>-> 空 | <表达式> | <表达式> ',' <数组元素列表>
       <可迭代结构> -> <表达式>
    */
    let array_elem_list = g.add_non_terminal("array_elem_list");
    let iter_expr = g.add_non_terminal("iter_expr");
    g.add_production(addr_elem, rhs!(l_brace, array_elem_list, r_brace));
    g.add_production(array_elem_list, rhs!());
    g.add_production(array_elem_list, rhs!(expr));
    g.add_production(array_elem_list, rhs!(expr, comma, array_elem_list));
    g.add_production(iter_expr, rhs!(expr));

    /*
       8.3 数组元素（依赖0.3、3.1，拓展0.3）
       <可取元素> -> <可取元素> '[' <表达式> ']'
    */
    g.add_production(addr_elem, rhs!(addr_elem, l_brace, expr, r_brace));

    /*
       Part9
       9.1 元组类型（依赖0.2，拓展0.2）
       <类型> -> '(' <元组类型内部> ')'
       <元组类型内部> -> 空 | <类型> ',' <类型列表>
       <类型列表> ->空 | <类型> | <类型> ',' <类型列表>
    */
    let tuple_ty_inner = g.add_non_terminal("tuple_ty_inner");
    let ty_list = g.add_non_terminal("ty_list");
    g.add_production(ty, rhs!(l_paren, tuple_ty_inner, r_paren));
    g.add_production(tuple_ty_inner, rhs!());
    g.add_production(tuple_ty_inner, rhs!(ty, comma, ty_list));
    g.add_production(ty_list, rhs!());
    g.add_production(ty_list, rhs!(ty));
    g.add_production(ty_list, rhs!(ty, comma, ty_list));

    /*
       9.2 元组表达式（依赖3.1、拓展0.3）
       <可取元素> -> '(' <元组赋值内部> ')'
       <元组赋值内部> -> 空 | <表达式> ',' <元组元素列表>
       <元组元素列表>->空 | <表达式> | <表达式> ',' <元组元素列表>
    */
    let tuple_expr_inner = g.add_non_terminal("tuple_expr_inner");
    let tuple_elem_list = g.add_non_terminal("tuple_elem_list");
    g.add_production(addr_elem, rhs!(l_paren, tuple_expr_inner, r_paren));
    g.add_production(tuple_expr_inner, rhs!());
    g.add_production(tuple_expr_inner, rhs!(expr, comma, tuple_elem_list));
    g.add_production(tuple_elem_list, rhs!());
    g.add_production(tuple_elem_list, rhs!(expr));
    g.add_production(tuple_elem_list, rhs!(expr, comma, tuple_elem_list));

    /*
       9.3 元组元素（依赖0.3，拓展0.3）
       <可取元素> -><可取元素> '.' <NUM>
    */
    g.add_production(addr_elem, rhs!(addr_elem, dot, num));

    g.set_start(program);
    let eof = g.add_terminal("eof");
    g.set_eof(eof);
    let terminals = Terminals {
        i32_,
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
        ident,
        literal_i32,
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
        eof,
    };
    let grammar = g.build()?;
    Ok(GrammarContext { grammar, terminals })
}
