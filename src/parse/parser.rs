use crate::lexer::{Token, TokenType};
use crate::logger::{ErrorType, Logger};
use core::panic;
use std::iter::Peekable;

// TODO add Tree Erorr so that the interpreter Print it later
// and remove panic!'s

#[derive(Debug, Clone)]
pub enum Tree {
    Number(usize),
    Ident(String),
    Empty(),
    String(String),
    BinOp(Box<Tree>, TokenType, Box<Tree>),
    CmpOp(Box<Tree>, TokenType, Box<Tree>),
    Inc(String),
    Dec(String),
    Exit(Box<Tree>),
    Let(String, Box<Tree>),
    Assign(String, Box<Tree>),
    // Args(Vec<Tree>),
    If {
        expr: Box<Tree>,
        body: Vec<Tree>,
        els: Vec<Tree>,
        els_ifs: Vec<Tree>,
    },
    ElsIf {
        expr: Box<Tree>,
        body: Vec<Tree>,
    },
    While {
        expr: Box<Tree>,
        body: Vec<Tree>,
    },
    For {
        var: String,
        expr: Box<Tree>,
        body: Vec<Tree>,
    },
}

// TODO change Every match iter.peek to while let
// TODO add curr_loc here too and change it every expr

pub struct Parser {
    tokens: Vec<Token>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens }
    }
    pub fn parse_tokens(&mut self) -> Vec<Tree> {
        let tokens_clone = self.tokens.clone();
        let mut iter: Peekable<std::slice::Iter<'_, Token>> = tokens_clone.iter().peekable();
        let mut trees = Vec::new();

        while iter.peek().is_some() {
            let tree = self.parse_expression(&mut iter);
            trees.push(tree);
        }
        trees
    }

    fn parse_expression(
        &mut self,
        iter: &mut std::iter::Peekable<std::slice::Iter<Token>>,
    ) -> Tree {
        let mut left = self.parse_term(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::Plus | TokenType::Minus => {
                    iter.next();
                    let right = self.parse_term(iter);
                    left = Tree::BinOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                TokenType::EquEqu | TokenType::NotEqu => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                TokenType::Greater | TokenType::GreatEqu | TokenType::Less | TokenType::LessEqu => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                TokenType::DDot => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                _ => break,
            }
        }

        left
    }

    fn parse_term(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Tree {
        let mut left = self.parse_factor(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::Multiply | TokenType::Divide => {
                    iter.next();
                    let right = self.parse_factor(iter);
                    left = Tree::BinOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                _ => break,
            }
        }
        left
    }
    fn parse_block(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Vec<Tree> {
        let mut body = vec![];
        match iter.peek().unwrap().token {
            TokenType::OpenCurly => {
                iter.next();
                while let Some(token) = iter.peek() {
                    match token.token {
                        TokenType::CloseCurly => {
                            iter.next();
                            break;
                        }
                        _ => body.push(self.parse_factor(iter)),
                    }
                }
            }
            _ => Logger::error("Expected {{", iter.peek().unwrap().loc, ErrorType::Parsing),
        }
        body
    }
    // TODO not used yet?
    //
    // fn parse_paren_expr(
    //     &mut self,
    //     iter: &mut std::iter::Peekable<std::slice::Iter<TokenType>>,
    // ) -> Tree {
    //     match iter.next().unwrap() {
    //         TokenType::OpenParen => {
    //             let expr = self.parse_expression(iter);
    //             iter.next();
    //             expr
    //         }
    //         _ => panic!("Expected ("),
    //     }
    // }

    // fn parse_args(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<TokenType>>) -> Tree {
    //     let mut vec_buffer: Vec<Tree> = vec![];
    //     while let Some(next) = iter.peek().cloned() {
    //         match next {
    //             TokenType::Comma => {
    //                 iter.next();
    //             }
    //             TokenType::CloseParen => {
    //                 iter.next();
    //                 break;
    //             }
    //             _ => vec_buffer.push(self.parse_expression(iter)),
    //         }
    //     }
    //     Tree::Args(vec_buffer)
    // }

    fn next_case(
        &mut self,
        iter: &mut Peekable<std::slice::Iter<'_, Token>>,
        els: &mut Vec<Tree>,
        els_ifs: &mut Vec<Tree>,
    ) {
        if !iter.peek().is_none() {
            match iter.peek().unwrap().token {
                TokenType::Els => {
                    iter.next();
                    if !els.is_empty() {
                        Logger::error(
                            "Unexpecte els statements",
                            iter.peek().unwrap().loc,
                            ErrorType::Parsing,
                        );
                        panic!("Excessive els statements")
                    }
                    *els = self.parse_block(iter);
                    self.next_case(iter, els, els_ifs);
                }
                TokenType::ElsIf => {
                    iter.next();
                    let expr = Box::new(self.parse_expression(iter));
                    let body = self.parse_block(iter);
                    els_ifs.push(Tree::ElsIf { expr, body });
                    self.next_case(iter, els, els_ifs);
                }
                _ => (),
            }
        }
    }

    fn parse_factor(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Tree {
        let it = iter.next().unwrap();
        match &it.token {
            TokenType::Number(num) => Tree::Number(*num),
            TokenType::Ident(string) => match iter.peek().unwrap().token {
                TokenType::Equal => {
                    iter.next();
                    let expr = self.parse_expression(iter);
                    Tree::Assign(string.to_string(), Box::new(expr))
                }
                TokenType::DPlus => {
                    iter.next();
                    Tree::Inc(string.to_string())
                }
                TokenType::DMinus => {
                    iter.next();
                    Tree::Dec(string.to_string())
                }
                _ => Tree::Ident(string.to_string()),
            },
            TokenType::String(string) => Tree::String(
                // i could use a crate for that  ig if i wanna use unicodes
                string
                    .to_string()
                    .replace("\\n", "\n")
                    .replace("\\t", "\t")
                    .replace("\\r", "\r")
                    .replace("\\\"", "\""),
            ),
            TokenType::Plus => self.parse_factor(iter),
            TokenType::Minus => {
                let factor = self.parse_factor(iter);
                Tree::BinOp(
                    Box::new(Tree::Number(0)),
                    TokenType::Minus,
                    Box::new(factor),
                )
            }
            TokenType::OpenParen => match iter.peek().unwrap().token {
                TokenType::CloseParen => {
                    iter.next();
                    let expr = Tree::Empty;
                    expr()
                }
                _ => {
                    let expr = self.parse_expression(iter);
                    match iter.next().unwrap().token {
                        TokenType::CloseParen => expr,
                        _ => {
                            Logger::error(
                                "Expected closing parenthesis",
                                it.loc,
                                ErrorType::Parsing,
                            );
                            panic!("Expected closing parenthesis")
                        }
                    }
                }
            },
            TokenType::Let => match &iter.next().unwrap().token {
                TokenType::Ident(var) => match iter.next().unwrap().token {
                    TokenType::Equal => {
                        let expr = self.parse_expression(iter);
                        Tree::Let(var.to_string(), Box::new(expr))
                    }
                    _ => {
                        Logger::error(
                            "Expected '=' after identifier in let statement",
                            it.loc,
                            ErrorType::Parsing,
                        );
                        panic!("Expected '=' after identifier in let statement")
                    }
                },
                _ => {
                    Logger::error(
                        "Expected identifier after 'let'",
                        it.loc,
                        ErrorType::Parsing,
                    );
                    panic!("Expected identifier after 'let'")
                }
            },
            TokenType::If => {
                let mut els = vec![];
                let mut els_ifs = vec![];
                let expr = Box::new(self.parse_expression(iter));
                let body = self.parse_block(iter);
                self.next_case(iter, &mut els, &mut els_ifs);
                Tree::If {
                    expr,
                    body,
                    els,
                    els_ifs,
                }
            }
            TokenType::While => {
                let expr = Box::new(self.parse_expression(iter));
                let body = self.parse_block(iter);
                Tree::While { expr, body }
            }
            TokenType::For => match &iter.next().unwrap().token {
                // TODO Syntax not Confirmed yet
                // for x -> 12 {}
                // for x..12 {}
                // for let x =-> 12
                TokenType::Ident(var) => match &iter.peek().unwrap().token {
                    TokenType::ThinArrow => {
                        iter.next();
                        let expr = Box::new(self.parse_expression(iter));
                        let body = self.parse_block(iter);
                        Tree::For {
                            var: var.to_string(),
                            expr,
                            body,
                        }
                    }
                    _ => panic!("Expected ->"),
                },
                TokenType::OpenParen => {
                    let expr = Box::new(self.parse_expression(iter));
                    iter.next();
                    let body = self.parse_block(iter);
                    Tree::While { expr, body }
                }
                _ => panic!("Expected (Expr) or Var -> expr..expr"),
            },
            TokenType::Exit => {
                let expr = self.parse_factor(iter);
                Tree::Exit(Box::new(expr))
            }
            TokenType::Els | TokenType::ElsIf => {
                Logger::error("Expected If statement first", it.loc, ErrorType::Parsing);
                panic!("Expected If statement first")
            }
            _ => panic!("Invalid factor"),
        }
    }
}
