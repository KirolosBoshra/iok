mod interpreter;
mod lexer;
mod logger;
mod object;
mod parser;

use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;
use std::{fs::File, io, io::Read, io::Write};
fn interpret_mode(interpreter: &mut Interpreter) {
    let mut input = String::new();
    loop {
        print!(">");
        input.clear();
        io::stdout().flush().unwrap();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        input = input.trim_end().to_string();

        if input.is_empty() {
            continue;
        }

        let mut lexer = Lexer::new(&input);
        let mut parser = Parser::new(lexer.tokenize());
        let ast = parser.parse_tokens();
        println!("{:?}", ast.last().unwrap());
        let obj = interpreter.interpret(ast.last().unwrap());
        println!("-> {}", obj);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        // eprintln!("Usages:\n./iok <file_path>");
        // std::process::exit(1);
        interpret_mode(&mut Interpreter::new());
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

    let mut lexer = Lexer::new(&input);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);

    let parsed_tree = parser.parse_tokens();

    let mut interpreter = Interpreter::new();

    parsed_tree.iter().for_each(|stmt| {
        interpreter.interpret(stmt);
    });
}
