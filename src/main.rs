use crate::compiler::Compiler;
use crate::compiler::source::SourceFile;

pub mod lexer;
pub mod parser;
mod my_grammar;
pub mod compiler;

fn main() {
    let compiler = Compiler::build().unwrap();
    let file = SourceFile::new("fn program_7_2(mut x:i32, mut y:i32) -> i32 { let mut t=x*x+x; t=t+x*y;
t}".to_string());
    if let Err(err) = compiler.compile(file) {
        eprintln!("Compile failed: {:?}", err);
    }
}
