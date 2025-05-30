mod interpreter;
mod lexer;
mod logger;
mod object;
mod parser;

use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;
use std::{env, fs::File, io, io::Read, io::Write, path::Path};
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
        let obj = interpreter.interpret(ast.last().unwrap());
        println!("-> {}", obj);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        let path = env::current_dir()
            .expect("Can't Access Dir")
            .to_str()
            .unwrap()
            .to_string();
        interpret_mode(&mut Interpreter::new(path));
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

    let dir_path = Path::new(file_name);
    let path = if let Ok(abs_path) = dir_path.canonicalize() {
        if let Some(parent) = abs_path.parent() {
            parent.to_str().unwrap().to_string()
        } else {
            String::from("/")
        }
    } else {
        String::from("/")
    };

    let mut interpreter = Interpreter::new(path);

    parsed_tree.iter().for_each(|stmt| {
        interpreter.interpret(stmt);
    });
}
