use crate::parser::grammar::{Grammar, GrammarBuilder};
/// 方便转换为Symbol，避免into调用过于冗长

macro_rules! rhs {
    ($($x:expr),* $(,)?) => {
        [$( $x.into() ),*]
    };
}

pub fn generate_my_grammar() -> Option<Grammar> {
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
    g.add_production(fn_sig, rhs!(fn_, ident, l_paren, param_list, r_paren, arrow, ty));
    g.add_production(return_stmt, rhs!(return_, stmt, semicolon));

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
    let expr = g.add_non_terminal("expr");
    g.add_production(expr, rhs!(assign_stmt));
    g.add_production(assign_stmt, rhs!(l_val, assignment, expr, semicolon));

    /*
        2.3 变量声明赋值语句（依赖2.0、3.1、拓展1.2）
        <语句>-> <变量声明赋值语句>
        <变量声明赋值语句> -> let <变量声明> '=' <表达式> ';'
     */
    let var_init_stmt = g.add_non_terminal("var_init_stmt");
    g.add_production(stmt, rhs!(var_init_stmt));
    g.add_production(var_init_stmt, rhs!(let_, var_decl, assignment, expr, semicolon));

    g.set_start(program);
    let eof = g.add_terminal("eof");
    g.set_eof(eof);

    match g.build() {
        Ok(grammar) => { Some(grammar) },
        Err(e) => {
            println!("Error building grammar: {:?}", e);
            None
        },
    }
}