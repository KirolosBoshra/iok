mod lexer;
mod parse;

use lexer::Lexer;
use parse::parser::Parser;
use std::{fs::File, io::Read};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usages:\n./iok <file_path>");
        std::process::exit(1);
    }

    let mut file_name = "main.iok";

    args.iter().for_each(|arg| {
        if arg.contains(".iok") {
            file_name = &arg
        }
    });

    let mut input = String::new();
    let mut file = File::open(file_name).expect(&format!("Can't open file {file_name}"));
    file.read_to_string(&mut input).expect("can't read file");

    let lexer = Lexer::new(input);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(&tokens);

    println!("{:?}\n\n", tokens);
    println!("{:?}", parser.parse_tokens());
}
