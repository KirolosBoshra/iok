mod interpreter;
mod lexer;
mod logger;
mod object;
mod parser;
mod std_native;

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
        interpret_mode(&mut Interpreter::new(path, None));
    }

    let args: Vec<String> = env::args().skip(1).collect();

    let mut std_path: Option<String> = None;
    let mut file_name: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--std" => {
                i += 1;
                if i < args.len() {
                    std_path = Some(args[i].clone());
                } else {
                    eprintln!("Expected a path after --std");
                    std::process::exit(1);
                }
            }
            arg if arg.ends_with(".iok") => {
                file_name = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    if file_name.is_none() {
        let path = env::current_dir()
            .expect("Can't Access Dir")
            .to_str()
            .unwrap()
            .to_string();
        let mut vm = Interpreter::new(path, std_path);
        interpret_mode(&mut vm);
    } else {
        let file_name = file_name.unwrap().clone();

        let mut input = String::new();
        let mut file =
            File::open(file_name.clone()).expect(&format!("Can't open file {}", file_name.clone()));
        file.read_to_string(&mut input).expect("can't read file");

        let mut lexer = Lexer::new(&input);
        let tokens = lexer.tokenize();

        let mut parser = Parser::new(tokens);

        let parsed_tree = parser.parse_tokens();

        let dir_path = Path::new(&file_name);
        let path = if let Ok(abs_path) = dir_path.canonicalize() {
            if let Some(parent) = abs_path.parent() {
                parent.to_str().unwrap().to_string()
            } else {
                String::from("/")
            }
        } else {
            String::from("/")
        };

        let mut interpreter = Interpreter::new(path, std_path);

        parsed_tree.iter().for_each(|stmt| {
            interpreter.interpret(stmt);
        });
    }
}
