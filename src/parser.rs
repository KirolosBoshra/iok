use crate::interner::intern;
use crate::lexer::{Loc, Token, TokenType};
use crate::logger::{ErrorType, Logger};
use rustc_hash::FxHashMap;
use std::iter::Peekable;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub enum Tree {
    Number(f64),
    Bool(bool),
    String(Rc<String>),
    List(Vec<Tree>),
    Ident(u32),
    Empty(),
    ListCall(Box<Tree>, Box<Tree>),
    FnCall {
        name: u32,
        args: Vec<Tree>,
    },
    MemberAccess {
        target: Box<Tree>, // variable
        member: Box<Tree>, // field or method()
    },

    Ret(Box<Tree>),
    Break,
    Continue,
    BinOp(Box<Tree>, TokenType, Box<Tree>),
    CmpOp(Box<Tree>, TokenType, Box<Tree>),
    Range(Box<Tree>, Box<Tree>),
    Let(u32, Box<Tree>),
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
        var: u32,
        expr: Box<Tree>,
        body: Vec<Tree>,
    },
    Match {
        expr: Box<Tree>,
        arms: Vec<(Vec<Tree>, Vec<Tree>)>,
        els: Vec<Tree>,
    },
    Fn {
        name: u32,
        args: Vec<Tree>,
        body: Vec<Tree>,
    },
    StructDef {
        name: u32,
        fields: Vec<Tree>,
        methods: Vec<Tree>,
    },
    StructInit {
        name: u32,
        fields: FxHashMap<u32, Tree>,
    },
    Import {
        path: Box<Tree>,
        alias: Option<u32>,
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
        let mut left = self.parse_or(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::DDot => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Tree::Range(Box::new(left), Box::new(right));
                }
                TokenType::DPlus | TokenType::DMinus => {
                    iter.next();
                    let op = match op.token {
                        TokenType::DPlus => TokenType::Plus,
                        _ => TokenType::Minus,
                    };
                    left = Tree::Assign(
                        Box::new(left.clone()),
                        Box::new(Tree::BinOp(Box::new(left), op, Box::new(Tree::Number(1.0)))),
                    );
                }
                TokenType::BitAnd | TokenType::BitOR | TokenType::Shl | TokenType::Shr => {
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

    fn parse_or(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<Token>>) -> Tree {
        let mut left = self.parse_and(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::Or => {
                    iter.next();
                    let right = self.parse_and(iter);
                    left = Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                _ => break,
            }
            self.prev_token = op.clone();
        }

        left
    }

    fn parse_and(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<Token>>) -> Tree {
        let mut left = self.parse_cmp(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::And => {
                    iter.next();
                    let right = self.parse_cmp(iter);
                    left = Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                _ => break,
            }
            self.prev_token = op.clone();
        }

        left
    }

    fn parse_cmp(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<Token>>) -> Tree {
        let mut left = self.parse_additive(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::EquEqu
                | TokenType::NotEqu
                | TokenType::Greater
                | TokenType::GreatEqu
                | TokenType::Less
                | TokenType::LessEqu => {
                    iter.next();
                    let right = self.parse_additive(iter);
                    left = Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right));
                }
                _ => break,
            }
            self.prev_token = op.clone();
        }

        left
    }

    fn parse_additive(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<Token>>) -> Tree {
        let mut left = self.parse_term(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::Plus | TokenType::Minus => {
                    iter.next();
                    let right = self.parse_term(iter);
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
                TokenType::Dot | TokenType::DColon => {
                    iter.next();
                    let member = Box::new(self.parse_factor(iter));
                    left = Tree::MemberAccess {
                        target: Box::new(left),
                        member,
                    };
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
                _ => Logger::error("Expected {{", Some(peek.loc), ErrorType::Parsing),
            }
        }
        body
    }

    // TODO Use this in all functions
    // Helper function to check and consume the expected token
    fn expect_token(
        &mut self,
        iter: &mut Peekable<std::slice::Iter<Token>>,
        expected: TokenType,
    ) -> Option<TokenType> {
        if let Some(&token) = iter.peek() {
            if std::mem::discriminant(&token.token) == std::mem::discriminant(&expected) {
                self.prev_token = token.clone();
                iter.next();
                return Some(token.token.clone());
            } else {
                Logger::error(
                    &format!(
                        "Expected token: {:?}, but found: {:?}",
                        expected, token.token
                    ),
                    Some(token.loc),
                    ErrorType::Parsing,
                );
            }
        } else {
            Logger::error(
                &format!("Expected token: {:?}, but reached end of input", expected),
                Some(self.prev_token.loc),
                ErrorType::Parsing,
            );
        }
        None
    }

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
                            Some(iter.peek().unwrap().loc),
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

    fn parse_delimited(
        &mut self,
        iter: &mut Peekable<std::slice::Iter<Token>>,
        close: TokenType,
        item: fn(&mut Self, &mut Peekable<std::slice::Iter<Token>>) -> Tree,
        err_msg: &str,
    ) -> Vec<Tree> {
        let mut items = vec![];
        while let Some(token) = iter.peek() {
            match token.token {
                TokenType::Comma => {
                    iter.next();
                }
                _ if token.token == close => {
                    iter.next();
                    return items;
                }
                _ => {
                    items.push(item(self, iter));
                }
            }
        }
        Logger::error(err_msg, Some(self.prev_token.loc), ErrorType::Parsing);
        items
    }

    fn parse_items(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Vec<Tree> {
        self.parse_delimited(
            iter,
            TokenType::CloseSquare,
            Parser::parse_factor,
            "Expected ] Or Items [..]",
        )
    }

    fn parse_args(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Vec<Tree> {
        if self.expect_token(iter, TokenType::OpenParen).is_some() {
            self.parse_delimited(
                iter,
                TokenType::CloseParen,
                Parser::parse_expression,
                "Expected ) Or Items (Args,..)",
            )
        } else {
            vec![]
        }
    }

    fn parse_struct_body(
        &mut self,
        iter: &mut Peekable<std::slice::Iter<Token>>,
    ) -> (Vec<Tree>, Vec<Tree>) {
        let mut fields = vec![];
        let mut methods = vec![];

        while iter
            .peek()
            .is_some_and(|t| t.token != TokenType::CloseCurly)
        {
            match iter.peek().unwrap().token {
                TokenType::Let => {
                    fields.push(self.parse_factor(iter));
                }
                TokenType::Fn => {
                    methods.push(self.parse_factor(iter));
                }

                _ => Logger::error(
                    "Unexpected Token",
                    Some(iter.peek().unwrap().loc),
                    ErrorType::Parsing,
                ),
            };
        }
        iter.next();

        (fields, methods)
    }

    fn parse_struct_fields(
        &mut self,
        iter: &mut Peekable<std::slice::Iter<Token>>,
    ) -> FxHashMap<u32, Tree> {
        let mut map = FxHashMap::default();
        while iter
            .peek()
            .is_some_and(|t| t.token != TokenType::CloseCurly)
        {
            if let Some(TokenType::Ident(field_name)) =
                self.expect_token(iter, TokenType::Ident(String::new()))
            {
                if self.expect_token(iter, TokenType::Colon).is_some() {
                    map.insert(intern(&field_name), self.parse_expression(iter));
                }
                if iter.peek().is_some_and(|t| t.token == TokenType::Comma) {
                    iter.next();
                    continue;
                }
            }
        }
        iter.next();
        map
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
                TokenType::Ident(string) => {
                    let ident = intern(string);
                    if let Some(p) = iter.peek() {
                        if p.token == TokenType::OpenParen {
                            let args = self.parse_args(iter);
                            return Tree::FnCall { name: ident, args };
                        }
                        if p.token == TokenType::OpenCurly {
                            let mut clone = iter.clone();
                            clone.next(); // skip the `{`
                                          // now the next token in `clone` should be Ident(fieldName)
                                          // and the one after that should be a Colon.
                            let is_struct_syntax = clone
                                .next()
                                .map(|t| match &t.token {
                                    TokenType::Ident(_) => true,
                                    _ => false,
                                })
                                .unwrap_or(false)
                                && clone
                                    .next()
                                    .map(|t| match &t.token {
                                        TokenType::Colon => true,
                                        _ => false,
                                    })
                                    .unwrap_or(false);

                            if is_struct_syntax {
                                // we really do have `Ident { field1: … }`
                                iter.next(); // consume the `{`
                                let fields = self.parse_struct_fields(iter);
                                return Tree::StructInit {
                                    name: ident,
                                    fields,
                                };
                            }
                        }
                    }
                    Tree::Ident(ident)
                }
                TokenType::String(string) => Tree::String(Rc::new(
                    string
                        .to_string()
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\r", "\r")
                        .replace("\\\"", "\""),
                )),
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
                TokenType::Ret => {
                    self.prev_token = it.clone();
                    Tree::Ret(Box::new(self.parse_expression(iter)))
                }
                TokenType::Break => Tree::Break,
                TokenType::Continue => Tree::Continue,
                TokenType::OpenParen => match iter.peek() {
                    Some(tok) if tok.token == TokenType::CloseParen => {
                        iter.next();
                        self.prev_token = it.clone();
                        let expr = Tree::Empty;
                        expr()
                    }
                    _ => {
                        let expr = self.parse_expression(iter);
                        match iter.next() {
                            Some(tok) if tok.token == TokenType::CloseParen => expr,
                            _ => {
                                Logger::error(
                                    "Expected closing parenthesis",
                                    Some(it.loc),
                                    ErrorType::Parsing,
                                );
                                Tree::Empty()
                            }
                        }
                    }
                },
                TokenType::Let => {
                    let Some(tok) = iter.next() else {
                        Logger::error(
                            "Expected identifier after 'let'",
                            Some(it.loc),
                            ErrorType::Parsing,
                        );
                        return Tree::Empty();
                    };
                    match &tok.token {
                        TokenType::Ident(var) => match iter.peek() {
                            Some(next) if next.token == TokenType::Equal => {
                                self.prev_token = (*next).clone();
                                iter.next();
                                let expr = self.parse_expression(iter);
                                Tree::Let(intern(var), Box::new(expr))
                            }
                            _ => Tree::Let(intern(var), Box::new(Tree::Empty())),
                        },
                        _ => {
                            Logger::error(
                                "Expected identifier after 'let'",
                                Some(it.loc),
                                ErrorType::Parsing,
                            );
                            Tree::Empty()
                        }
                    }
                }
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
                TokenType::For => {
                    let Some(tok) = iter.next() else {
                        Logger::error(
                            "Expected Var -> Expr..Expr or Var -> List",
                            Some(it.loc),
                            ErrorType::Parsing,
                        );
                        return Tree::Empty();
                    };
                    match &tok.token {
                        TokenType::Ident(var) => match iter.peek() {
                            Some(next) if next.token == TokenType::ThinArrow => {
                                iter.next();
                                let expr = Box::new(self.parse_expression(iter));
                                let body = self.parse_block(iter);
                                self.prev_token = it.clone();
                                Tree::For {
                                    var: intern(var),
                                    expr,
                                    body,
                                }
                            }
                            _ => {
                                Logger::error("Expected ->", Some(it.loc), ErrorType::Parsing);
                                Tree::Empty()
                            }
                        },
                        _ => {
                            Logger::error(
                                "Expected Var -> Expr..Expr or Var -> List",
                                Some(it.loc),
                                ErrorType::Parsing,
                            );
                            Tree::Empty()
                        }
                    }
                }
                TokenType::Fn => {
                    if let Some(TokenType::Ident(name)) =
                        self.expect_token(iter, TokenType::Ident(String::new()))
                    {
                        let args = self.parse_args(iter);
                        let mut body = vec![];
                        if self.expect_token(iter, TokenType::FatArrow).is_some() {
                            if let Some(next) = iter.peek() {
                                match next.token {
                                    TokenType::OpenCurly => {
                                        body = self.parse_block(iter);
                                    }
                                    _ => body.push(self.parse_expression(iter)),
                                }
                            }
                        }
                        return Tree::Fn {
                            name: intern(&name),
                            args,
                            body,
                        };
                    };
                    Tree::Empty()
                }

                TokenType::Struct => {
                    if let Some(TokenType::Ident(name)) =
                        self.expect_token(iter, TokenType::Ident(String::new()))
                    {
                        if self.expect_token(iter, TokenType::OpenCurly).is_some() {
                            let (fields, methods) = self.parse_struct_body(iter);
                            return Tree::StructDef {
                                name: intern(&name),
                                fields,
                                methods,
                            };
                        }
                    } else {
                        Logger::error("Expected Struct Name", Some(it.loc), ErrorType::Parsing);
                    }
                    Tree::Empty()
                }

                TokenType::Match => {
                    let expr = Box::new(self.parse_expression(iter));

                    if self.expect_token(iter, TokenType::OpenCurly).is_none() {
                        return Tree::Empty();
                    }

                    let mut arms: Vec<(Vec<Tree>, Vec<Tree>)> = vec![];
                    let mut els: Vec<Tree> = vec![];

                    while let Some(peek) = iter.peek() {
                        let arm_loc = peek.loc;
                        if peek.token == TokenType::CloseCurly {
                            iter.next();
                            break;
                        }
                        let mut patterns = vec![self.parse_expression(iter)];
                        while iter.peek().is_some_and(|t| t.token == TokenType::Comma) {
                            iter.next();
                            patterns.push(self.parse_expression(iter));
                        }
                        let mut body = vec![];
                        if self.expect_token(iter, TokenType::FatArrow).is_none() {
                            break;
                        }
                        if let Some(next) = iter.peek() {
                            match next.token {
                                TokenType::OpenCurly => body = self.parse_block(iter),
                                _ => body.push(self.parse_expression(iter)),
                            }
                        }
                        if patterns
                            .iter()
                            .any(|p| matches!(p, Tree::Ident(id) if *id == intern("_")))
                        {
                            if !els.is_empty() {
                                Logger::error(
                                    "Duplicate wildcard arm",
                                    Some(arm_loc),
                                    ErrorType::Parsing,
                                );
                            }
                            els = body;
                        } else {
                            arms.push((patterns, body));
                        }
                    }
                    self.prev_token = it.clone();
                    Tree::Match { expr, arms, els }
                }

                TokenType::Import => {
                    let path = Box::new(self.parse_expression(iter));
                    let mut alias = None;
                    if iter.peek().is_some() && iter.peek().unwrap().token == TokenType::As {
                        iter.next();
                        if let TokenType::Ident(ref name) = iter.peek().unwrap().token {
                            iter.next();
                            alias = Some(intern(name));
                        }
                    }
                    Tree::Import { path, alias }
                }

                TokenType::Els | TokenType::ElsIf => {
                    Logger::error(
                        "Expected If statement first",
                        Some(it.loc),
                        ErrorType::Parsing,
                    );
                    Tree::Empty()
                }
                _ => {
                    Logger::error(
                        &format!("Invalid Token {:?}", it.token),
                        Some(it.loc),
                        ErrorType::Parsing,
                    );
                    Tree::Empty()
                }
            }
        } else {
            Logger::error(
                "Expected Statement",
                Some(self.prev_token.loc),
                ErrorType::Parsing,
            );
            Tree::Empty()
        }
    }
}
