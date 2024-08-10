use crate::lexer::{Loc, Token, TokenType};
use crate::logger::{ErrorType, Logger};
use std::iter::Peekable;

#[derive(Debug, Clone)]
pub enum Tree {
    Number(f64),
    Bool(bool),
    String(String),
    List(Vec<Tree>),
    Ident(String),
    ListCall(Box<Tree>, Box<Tree>),
    Empty(),
    BinOp(Box<Tree>, TokenType, Box<Tree>),
    CmpOp(Box<Tree>, TokenType, Box<Tree>),
    Range(Box<Tree>, Box<Tree>),
    Exit(Box<Tree>),
    Dbg(Box<Tree>),
    Let(String, Box<Tree>),
    Assign(Box<Tree>, Box<Tree>),
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

pub struct Parser {
    pub tokens: Vec<Token>,
    prev_token: Token,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            prev_token: Token {
                token: TokenType::Null,
                loc: Loc { x: 0, y: 0 },
            },
        }
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
                    left = Tree::Range(Box::new(left), Box::new(right));
                }
                TokenType::DPlus => {
                    iter.next();
                    left = Tree::Assign(
                        Box::new(left.clone()),
                        Box::new(Tree::BinOp(
                            Box::new(left),
                            TokenType::Plus,
                            Box::new(Tree::Number(1.0)),
                        )),
                    );
                }
                TokenType::DMinus => {
                    iter.next();
                    left = Tree::Assign(
                        Box::new(left.clone()),
                        Box::new(Tree::BinOp(
                            Box::new(left),
                            TokenType::Minus,
                            Box::new(Tree::Number(1.0)),
                        )),
                    );
                }
                TokenType::BitAnd | TokenType::BitOR => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Tree::BinOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                TokenType::Shl | TokenType::Shr => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Tree::BinOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                _ => break,
            }
            self.prev_token = op.clone();
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
                TokenType::Equal => {
                    iter.next();
                    let expr = self.parse_expression(iter);
                    left = Tree::Assign(Box::new(left), Box::new(expr));
                }
                TokenType::PlusEqu => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Tree::Assign(
                        Box::new(left.clone()),
                        Box::new(Tree::BinOp(
                            Box::new(left),
                            TokenType::Plus,
                            Box::new(right),
                        )),
                    );
                }
                TokenType::And | TokenType::Or => {
                    iter.next();
                    let right = self.parse_factor(iter);
                    left = Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                TokenType::OpenSquare => {
                    iter.next();
                    while let Some(peek) = iter.peek().clone() {
                        match peek.token {
                            TokenType::CloseSquare => {
                                iter.next();
                                break;
                            }
                            _ => {
                                let index = self.parse_expression(iter);
                                left = Tree::ListCall(Box::new(left), Box::new(index));
                            }
                        }
                    }
                }
                _ => break,
            }
            self.prev_token = op.clone();
        }
        left
    }
    fn parse_block(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Vec<Tree> {
        let mut body = vec![];
        if let Some(peek) = iter.peek() {
            match peek.token {
                TokenType::OpenCurly => {
                    iter.next();
                    while let Some(token) = iter.peek() {
                        match token.token {
                            TokenType::CloseCurly => {
                                iter.next();
                                break;
                            }
                            _ => {
                                let expr = self.parse_expression(iter);
                                body.push(expr);
                            }
                        }
                    }
                }
                _ => Logger::error("Expected {{", peek.loc, ErrorType::Parsing),
            }
        }
        body
    }

    // Helper function to check and consume the expected token
    // fn expect_token(
    //     &mut self,
    //     iter: &mut Peekable<std::slice::Iter<Token>>,
    //     expected: TokenType,
    // ) -> bool {
    //     if let Some(token) = iter.peek() {
    //         if token.token == expected {
    //             iter.next(); // Consume the expected token
    //             return true;
    //         } else {
    //             Logger::error(
    //                 &format!(
    //                     "Expected token: {:?}, but found: {:?}",
    //                     expected, token.token
    //                 ),
    //                 token.loc,
    //                 ErrorType::Parsing,
    //             );
    //         }
    //     } else {
    //         Logger::error(
    //             &format!("Expected token: {:?}, but reached end of input", expected),
    //             self.prev_token.loc,
    //             ErrorType::Parsing,
    //         );
    //     }
    //     false
    // }

    fn next_case(
        &mut self,
        iter: &mut Peekable<std::slice::Iter<'_, Token>>,
        els: &mut Vec<Tree>,
        els_ifs: &mut Vec<Tree>,
    ) {
        if let Some(peek) = iter.peek() {
            match peek.token {
                TokenType::Els => {
                    iter.next();
                    if !els.is_empty() {
                        Logger::error(
                            "Unexpected els statements",
                            iter.peek().unwrap().loc,
                            ErrorType::Parsing,
                        );
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

    fn parse_items(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Vec<Tree> {
        let mut items = vec![];
        while let Some(item) = iter.peek() {
            match item.token {
                TokenType::CloseSquare => {
                    iter.next();
                    return items;
                }
                TokenType::Comma => {
                    iter.next();
                }
                _ => {
                    items.push(self.parse_factor(iter));
                }
            }
        }
        Logger::error(
            "Expected ] Or Items [..]",
            self.prev_token.loc,
            ErrorType::Parsing,
        );
        items
    }

    fn parse_factor(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Tree {
        if let Some(it) = iter.next() {
            match &it.token {
                TokenType::Number(num) => Tree::Number(*num),
                TokenType::Bool(b) => Tree::Bool(*b),
                TokenType::Null => Tree::Empty(),
                TokenType::Bang => {
                    let expr = self.parse_expression(iter);
                    Tree::CmpOp(Box::new(expr), TokenType::Bang, Box::new(Tree::Empty()))
                }
                TokenType::Ident(string) => Tree::Ident(string.to_string()),
                TokenType::String(string) => Tree::String(
                    // i could use a crate for that  ig if i wanna use unicodes
                    string
                        .to_string()
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\r", "\r")
                        .replace("\\\"", "\""),
                ),
                TokenType::OpenSquare => {
                    let items = self.parse_items(iter);
                    Tree::List(items)
                }
                TokenType::Plus => self.parse_factor(iter),
                TokenType::Minus => {
                    let factor = self.parse_factor(iter);
                    Tree::BinOp(
                        Box::new(Tree::Number(0.0)),
                        TokenType::Minus,
                        Box::new(factor),
                    )
                }
                TokenType::OpenParen => match iter.peek().unwrap().token {
                    TokenType::CloseParen => {
                        iter.next();
                        self.prev_token = it.clone();
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
                                Tree::Empty()
                            }
                        }
                    }
                },
                TokenType::Let => match &iter
                    .next()
                    .unwrap_or(&Token {
                        token: TokenType::Null,
                        loc: it.loc,
                    })
                    .token
                {
                    TokenType::Ident(var) => {
                        if let Some(next) = iter.next() {
                            match next.token {
                                TokenType::Equal => {
                                    self.prev_token = next.clone();
                                    let expr = self.parse_expression(iter);
                                    return Tree::Let(var.to_string(), Box::new(expr));
                                }
                                _ => {
                                    Logger::error(
                                        "Expected '=' after identifier in let statement",
                                        it.loc,
                                        ErrorType::Parsing,
                                    );
                                    return Tree::Empty();
                                }
                            }
                        } else {
                            Logger::error(
                                "Expected '=' after identifier in let statement",
                                it.loc,
                                ErrorType::Parsing,
                            );
                            return Tree::Empty();
                        }
                    }
                    _ => {
                        Logger::error(
                            "Expected identifier after 'let'",
                            it.loc,
                            ErrorType::Parsing,
                        );
                        Tree::Empty()
                    }
                },
                TokenType::If => {
                    let mut els = vec![];
                    let mut els_ifs = vec![];
                    let expr = Box::new(self.parse_expression(iter));
                    let body = self.parse_block(iter);
                    self.next_case(iter, &mut els, &mut els_ifs);
                    self.prev_token = it.clone();
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
                    self.prev_token = it.clone();
                    Tree::While { expr, body }
                }
                TokenType::For => match &iter.next().unwrap().token {
                    // TODO Change Syntax to
                    // for i -> 0..12 {}
                    TokenType::Ident(var) => match &iter.peek().unwrap().token {
                        TokenType::ThinArrow => {
                            iter.next();
                            let expr = Box::new(self.parse_expression(iter));
                            let body = self.parse_block(iter);
                            self.prev_token = it.clone();
                            Tree::For {
                                var: var.to_string(),
                                expr,
                                body,
                            }
                        }
                        _ => {
                            Logger::error("Expected ->", it.loc, ErrorType::Parsing);
                            Tree::Empty()
                        }
                    },
                    _ => {
                        Logger::error(
                            "Expected Var -> Expr..Expr or Var -> List",
                            it.loc,
                            ErrorType::Parsing,
                        );
                        Tree::Empty()
                    }
                },
                TokenType::Exit => {
                    let expr = self.parse_factor(iter);
                    Tree::Exit(Box::new(expr))
                }
                // TODO TMP function
                TokenType::Dbg => {
                    let expr = self.parse_factor(iter);
                    Tree::Dbg(Box::new(expr))
                }
                TokenType::Els | TokenType::ElsIf => {
                    Logger::error("Expected If statement first", it.loc, ErrorType::Parsing);
                    Tree::Empty()
                }
                _ => {
                    Logger::error(
                        &format!("Invalid Token {:?}", it.token),
                        it.loc,
                        ErrorType::Parsing,
                    );
                    Tree::Empty()
                }
            }
        } else {
            Logger::error(
                "Expected Statement",
                self.prev_token.loc,
                ErrorType::Parsing,
            );
            Tree::Empty()
        }
    }
}
