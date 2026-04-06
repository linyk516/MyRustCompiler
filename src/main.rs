use crate::compiler::Compiler;
use crate::compiler::source::SourceFile;

pub mod lexer;
pub mod parser;
mod my_grammar;
pub mod compiler;

fn main() {
    let compiler = Compiler::build().unwrap();
    let file = SourceFile::new("fn program_7_2(mut x:i32, mut y:i32) -> i32 { let mut t=x*x+x; t=t+x*y; t}".to_string());
    match compiler.compile(file) {
        Err(err) => eprintln!("Compile failed: {:?}", err),
        Ok(output) => println!(
            "Compile succeeded. CST has {} nodes:\n{}",
            output.cst().nodes.len(),
            compiler.display_cst(&output),
        ),
    }
}
