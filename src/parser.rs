use crate::interner::intern;
use crate::lexer::{Loc, Token, TokenType};
use crate::logger::{ErrorType, Logger};
use rustc_hash::FxHashMap;
use std::iter::Peekable;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub tree: Tree,
    pub loc: Loc,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tree {
    Number(f64),
    Bool(bool),
    String(Rc<String>),
    List(Vec<Node>),
    Ident(u32),
    Empty(),
    ImmCall {
        callee: Box<Node>,
        args: Vec<Node>,
    },
    ListCall(Box<Node>, Box<Node>),
    FnCall {
        name: u32,
        args: Vec<Node>,
    },
    MemberAccess {
        target: Box<Node>, // variable
        member: Box<Node>, // field or method()
    },

    Ret(Box<Node>),
    Break,
    Continue,
    BinOp(Box<Node>, TokenType, Box<Node>),
    CmpOp(Box<Node>, TokenType, Box<Node>),
    Range(Box<Node>, Box<Node>),
    Let(u32, Box<Node>),
    Assign(Box<Node>, Box<Node>),
    If {
        expr: Box<Node>,
        body: Vec<Node>,
        els: Vec<Node>,
        els_ifs: Vec<Node>,
    },
    ElsIf {
        expr: Box<Node>,
        body: Vec<Node>,
    },
    While {
        expr: Box<Node>,
        body: Vec<Node>,
    },
    For {
        var: u32,
        expr: Box<Node>,
        body: Vec<Node>,
    },
    Match {
        expr: Box<Node>,
        arms: Vec<(Vec<Node>, Vec<Node>)>,
        els: Vec<Node>,
    },
    Fn {
        name: Option<u32>,
        args: Vec<Node>,
        body: Vec<Node>,
    },
    StructDef {
        name: u32,
        fields: Vec<Node>,
        methods: Vec<Node>,
    },
    StructInit {
        name: u32,
        fields: FxHashMap<u32, Node>,
    },
    Import {
        path: Box<Node>,
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
    pub fn parse_tokens(&mut self) -> Vec<Node> {
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
    ) -> Node {
        let mut left = self.parse_or(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::DDot => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Node {
                        tree: Tree::Range(Box::new(left), Box::new(right)),
                        loc: op.loc,
                    };
                }
                TokenType::DPlus | TokenType::DMinus => {
                    iter.next();
                    let op_loc = op.loc;
                    let op = match op.token {
                        TokenType::DPlus => TokenType::Plus,
                        _ => TokenType::Minus,
                    };
                    left = Node {
                        tree: Tree::Assign(
                            Box::new(left.clone()),
                            Box::new(Node {
                                tree: Tree::BinOp(
                                    Box::new(left),
                                    op,
                                    Box::new(Node {
                                        tree: Tree::Number(1.0),
                                        loc: op_loc,
                                    }),
                                ),
                                loc: op_loc,
                            }),
                        ),
                        loc: op_loc,
                    };
                }
                TokenType::BitAnd | TokenType::BitOR | TokenType::Shl | TokenType::Shr => {
                    iter.next();
                    let right = self.parse_expression(iter);
                    left = Node {
                        tree: Tree::BinOp(Box::new(left), op.token.clone(), Box::new(right)),
                        loc: op.loc,
                    };
                }

                _ => break,
            }
            self.prev_token = op.clone();
        }

        left
    }

    fn parse_or(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<Token>>) -> Node {
        let mut left = self.parse_and(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::Or => {
                    iter.next();
                    let right = self.parse_and(iter);
                    left = Node {
                        tree: Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right)),
                        loc: op.loc,
                    };
                }
                _ => break,
            }
            self.prev_token = op.clone();
        }

        left
    }

    fn parse_and(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<Token>>) -> Node {
        let mut left = self.parse_cmp(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::And => {
                    iter.next();
                    let right = self.parse_cmp(iter);
                    left = Node {
                        tree: Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right)),
                        loc: op.loc,
                    };
                }
                _ => break,
            }
            self.prev_token = op.clone();
        }

        left
    }

    fn parse_cmp(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<Token>>) -> Node {
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
                    left = Node {
                        tree: Tree::CmpOp(Box::new(left), op.token.clone(), Box::new(right)),
                        loc: op.loc,
                    };
                }
                _ => break,
            }
            self.prev_token = op.clone();
        }

        left
    }

    fn parse_additive(&mut self, iter: &mut std::iter::Peekable<std::slice::Iter<Token>>) -> Node {
        let mut left = self.parse_term(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::Plus | TokenType::Minus => {
                    iter.next();
                    let right = self.parse_term(iter);
                    left = Node {
                        tree: Tree::BinOp(Box::new(left), op.token.clone(), Box::new(right)),
                        loc: op.loc,
                    };
                }

                _ => break,
            }
            self.prev_token = op.clone();
        }

        left
    }

    fn parse_term(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Node {
        let mut left = self.parse_power(iter);

        while let Some(op) = iter.peek().cloned() {
            match op.token {
                TokenType::Multiply | TokenType::Divide | TokenType::Percent => {
                    iter.next();
                    let right = self.parse_term(iter);
                    left = Node {
                        tree: Tree::BinOp(Box::new(left), op.token.clone(), Box::new(right)),
                        loc: op.loc,
                    };
                }
                TokenType::DMultiply => {
                    iter.next();
                    let right = self.parse_term(iter);
                    left = Node {
                        tree: Tree::BinOp(Box::new(left), TokenType::DMultiply, Box::new(right)),
                        loc: op.loc,
                    };
                }
                TokenType::Equal => {
                    iter.next();
                    let expr = self.parse_expression(iter);
                    left = Node {
                        tree: Tree::Assign(Box::new(left), Box::new(expr)),
                        loc: op.loc,
                    };
                }

                TokenType::PlusEqu
                | TokenType::MinusEqu
                | TokenType::MultiplyEqu
                | TokenType::DivideEqu
                | TokenType::PercentEqu
                | TokenType::PowerEqu => {
                    iter.next();
                    let op_loc = op.loc;
                    let op = match op.token {
                        TokenType::PlusEqu => TokenType::Plus,
                        TokenType::MinusEqu => TokenType::Minus,
                        TokenType::MultiplyEqu => TokenType::Multiply,
                        TokenType::DivideEqu => TokenType::Divide,
                        TokenType::PercentEqu => TokenType::Percent,
                        _ => TokenType::DMultiply,
                    };
                    let right = self.parse_expression(iter);
                    left = Node {
                        tree: Tree::Assign(
                            Box::new(left.clone()),
                            Box::new(Node {
                                tree: Tree::BinOp(Box::new(left), op, Box::new(right)),
                                loc: op_loc,
                            }),
                        ),
                        loc: op_loc,
                    };
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
                                left = Node {
                                    tree: Tree::ListCall(Box::new(left), Box::new(index)),
                                    loc: op.loc,
                                };
                            }
                        }
                    }
                }
                TokenType::OpenParen => {
                    if matches!(&left.tree, Tree::Fn { name: None, .. }) {
                        let args = self.parse_args(iter);
                        left = Node {
                            tree: Tree::ImmCall {
                                callee: Box::new(left),
                                args,
                            },
                            loc: op.loc,
                        };
                    } else {
                        break;
                    }
                }
                TokenType::Dot | TokenType::DColon => {
                    iter.next();
                    let member = Box::new(self.parse_factor(iter));
                    left = Node {
                        tree: Tree::MemberAccess {
                            target: Box::new(left),
                            member,
                        },
                        loc: op.loc,
                    };
                }

                _ => break,
            }
            self.prev_token = op.clone();
        }
        left
    }
    fn parse_block(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Vec<Node> {
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
        els: &mut Vec<Node>,
        els_ifs: &mut Vec<Node>,
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
                    let elsif_loc = peek.loc;
                    iter.next();
                    let expr = Box::new(self.parse_expression(iter));
                    let body = self.parse_block(iter);
                    els_ifs.push(Node {
                        tree: Tree::ElsIf { expr, body },
                        loc: elsif_loc,
                    });
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
        item: fn(&mut Self, &mut Peekable<std::slice::Iter<Token>>) -> Node,
        err_msg: &str,
    ) -> Vec<Node> {
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

    fn parse_items(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Vec<Node> {
        self.parse_delimited(
            iter,
            TokenType::CloseSquare,
            Parser::parse_factor,
            "Expected ] Or Items [..]",
        )
    }

    fn parse_args(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Vec<Node> {
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

    fn is_arrow(&self, iter: &mut Peekable<std::slice::Iter<Token>>) -> bool {
        iter.peek().is_some_and(|t| t.token == TokenType::FatArrow)
    }

    fn parse_arrow_fn_body(
        &mut self,
        iter: &mut Peekable<std::slice::Iter<Token>>,
    ) -> Vec<Node> {
        let mut body = vec![];
        if let Some(next) = iter.peek() {
            match next.token {
                TokenType::OpenCurly => body = self.parse_block(iter),
                _ => body.push(self.parse_expression(iter)),
            }
        }
        body
    }

    fn parse_struct_body(
        &mut self,
        iter: &mut Peekable<std::slice::Iter<Token>>,
    ) -> (Vec<Node>, Vec<Node>) {
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
    ) -> FxHashMap<u32, Node> {
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

    fn parse_power(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Node {
        let mut left = self.parse_factor(iter);

        while let Some(op) = iter.peek().cloned() {
            if op.token != TokenType::DMultiply {
                break;
            }
            iter.next();
            let right = self.parse_power(iter);
            left = Node {
                tree: Tree::BinOp(Box::new(left), TokenType::DMultiply, Box::new(right)),
                loc: op.loc,
            };
            self.prev_token = op.clone();
        }
        left
    }

    fn parse_factor(&mut self, iter: &mut Peekable<std::slice::Iter<Token>>) -> Node {
        if let Some(it) = iter.next() {
            match &it.token {
                TokenType::Number(num) => Node {
                    tree: Tree::Number(*num),
                    loc: it.loc,
                },
                TokenType::Bool(b) => Node {
                    tree: Tree::Bool(*b),
                    loc: it.loc,
                },
                TokenType::Null => Node {
                    tree: Tree::Empty(),
                    loc: it.loc,
                },
                TokenType::Bang => {
                    let expr = self.parse_expression(iter);
                    Node {
                        tree: Tree::CmpOp(
                            Box::new(expr),
                            TokenType::Bang,
                            Box::new(Node {
                                tree: Tree::Empty(),
                                loc: it.loc,
                            }),
                        ),
                        loc: it.loc,
                    }
                }
                TokenType::BitNot => {
                    let expr = self.parse_expression(iter);
                    Node {
                        tree: Tree::CmpOp(
                            Box::new(expr),
                            TokenType::BitNot,
                            Box::new(Node {
                                tree: Tree::Empty(),
                                loc: it.loc,
                            }),
                        ),
                        loc: it.loc,
                    }
                }
                TokenType::Ident(string) => {
                    let ident = intern(string);
                    if let Some(p) = iter.peek() {
                        if p.token == TokenType::OpenParen {
                            let args = self.parse_args(iter);
                            return Node {
                                tree: Tree::FnCall { name: ident, args },
                                loc: it.loc,
                            };
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
                                return Node {
                                    tree: Tree::StructInit {
                                        name: ident,
                                        fields,
                                    },
                                    loc: it.loc,
                                };
                            }
                        }
                    }
                    Node {
                        tree: Tree::Ident(ident),
                        loc: it.loc,
                    }
                }
                TokenType::String(string) => Node {
                    tree: Tree::String(Rc::new(
                        string
                            .to_string()
                            .replace("\\n", "\n")
                            .replace("\\t", "\t")
                            .replace("\\r", "\r")
                            .replace("\\\"", "\""),
                    )),
                    loc: it.loc,
                },
                TokenType::OpenSquare => {
                    let items = self.parse_items(iter);
                    Node {
                        tree: Tree::List(items),
                        loc: it.loc,
                    }
                }
                TokenType::Plus => self.parse_factor(iter),
                TokenType::Minus => {
                    let factor = self.parse_factor(iter);
                    Node {
                        tree: Tree::BinOp(
                            Box::new(Node {
                                tree: Tree::Number(0.0),
                                loc: it.loc,
                            }),
                            TokenType::Minus,
                            Box::new(factor),
                        ),
                        loc: it.loc,
                    }
                }
                TokenType::Ret => {
                    self.prev_token = it.clone();
                    Node {
                        tree: Tree::Ret(Box::new(self.parse_expression(iter))),
                        loc: it.loc,
                    }
                }
                TokenType::Break => Node {
                    tree: Tree::Break,
                    loc: it.loc,
                },
                TokenType::Continue => Node {
                    tree: Tree::Continue,
                    loc: it.loc,
                },
                TokenType::OpenParen => {
                    match iter.peek() {
                        Some(tok) if tok.token == TokenType::CloseParen => {
                            iter.next();
                            self.prev_token = it.clone();
                            if self.is_arrow(iter) {
                                iter.next();
                                let body = self.parse_arrow_fn_body(iter);
                                Node {
                                    tree: Tree::Fn {
                                        name: None,
                                        args: vec![],
                                        body,
                                    },
                                    loc: it.loc,
                                }
                            } else {
                                Node {
                                    tree: Tree::Empty(),
                                    loc: it.loc,
                                }
                            }
                        }
                        _ => {
                            let first = self.parse_expression(iter);
                            match iter.next() {
                                Some(tok) if tok.token == TokenType::CloseParen => {
                                    if self.is_arrow(iter) {
                                        iter.next();
                                        let body = self.parse_arrow_fn_body(iter);
                                        Node {
                                            tree: Tree::Fn {
                                                name: None,
                                                args: vec![first],
                                                body,
                                            },
                                            loc: it.loc,
                                        }
                                    } else {
                                        first
                                    }
                                }
                                Some(tok) if tok.token == TokenType::Comma => {
                                    let mut args = vec![first];
                                    loop {
                                        match iter.peek().map(|t| &t.token) {
                                            Some(&TokenType::Comma) => {
                                                iter.next();
                                            }
                                            Some(&TokenType::CloseParen) => {
                                                iter.next();
                                                break;
                                            }
                                            Some(_) => {
                                                args.push(self.parse_expression(iter));
                                            }
                                            None => break,
                                        }
                                    }
                                    if self.is_arrow(iter) {
                                        iter.next();
                                        let body = self.parse_arrow_fn_body(iter);
                                        Node {
                                            tree: Tree::Fn {
                                                name: None,
                                                args,
                                                body,
                                            },
                                            loc: it.loc,
                                        }
                                    } else {
                                        Logger::error(
                                            "Expected closing parenthesis",
                                            Some(it.loc),
                                            ErrorType::Parsing,
                                        );
                                        Node {
                                            tree: Tree::Empty(),
                                            loc: it.loc,
                                        }
                                    }
                                }
                                _ => {
                                    Logger::error(
                                        "Expected closing parenthesis",
                                        Some(it.loc),
                                        ErrorType::Parsing,
                                    );
                                    Node {
                                        tree: Tree::Empty(),
                                        loc: it.loc,
                                    }
                                }
                            }
                        }
                    }
                }
                TokenType::Let => {
                    let Some(tok) = iter.next() else {
                        Logger::error(
                            "Expected identifier after 'let'",
                            Some(it.loc),
                            ErrorType::Parsing,
                        );
                        return Node {
                            tree: Tree::Empty(),
                            loc: it.loc,
                        };
                    };
                    match &tok.token {
                        TokenType::Ident(var) => match iter.peek() {
                            Some(next) if next.token == TokenType::Equal => {
                                self.prev_token = (*next).clone();
                                iter.next();
                                let expr = self.parse_expression(iter);
                                Node {
                                    tree: Tree::Let(intern(var), Box::new(expr)),
                                    loc: it.loc,
                                }
                            }
                            _ => Node {
                                tree: Tree::Let(
                                    intern(var),
                                    Box::new(Node {
                                        tree: Tree::Empty(),
                                        loc: it.loc,
                                    }),
                                ),
                                loc: it.loc,
                            },
                        },
                        _ => {
                            Logger::error(
                                "Expected identifier after 'let'",
                                Some(it.loc),
                                ErrorType::Parsing,
                            );
                            Node {
                                tree: Tree::Empty(),
                                loc: it.loc,
                            }
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
                    Node {
                        tree: Tree::If {
                            expr,
                            body,
                            els,
                            els_ifs,
                        },
                        loc: it.loc,
                    }
                }
                TokenType::While => {
                    let expr = Box::new(self.parse_expression(iter));
                    let body = self.parse_block(iter);
                    self.prev_token = it.clone();
                    Node {
                        tree: Tree::While { expr, body },
                        loc: it.loc,
                    }
                }
                TokenType::For => {
                    let Some(tok) = iter.next() else {
                        Logger::error(
                            "Expected Var -> Expr..Expr or Var -> List",
                            Some(it.loc),
                            ErrorType::Parsing,
                        );
                        return Node {
                            tree: Tree::Empty(),
                            loc: it.loc,
                        };
                    };
                    match &tok.token {
                        TokenType::Ident(var) => match iter.peek() {
                            Some(next) if next.token == TokenType::ThinArrow => {
                                iter.next();
                                let expr = Box::new(self.parse_expression(iter));
                                let body = self.parse_block(iter);
                                self.prev_token = it.clone();
                                Node {
                                    tree: Tree::For {
                                        var: intern(var),
                                        expr,
                                        body,
                                    },
                                    loc: it.loc,
                                }
                            }
                            _ => {
                                Logger::error("Expected ->", Some(it.loc), ErrorType::Parsing);
                                Node {
                                    tree: Tree::Empty(),
                                    loc: it.loc,
                                }
                            }
                        },
                        _ => {
                            Logger::error(
                                "Expected Var -> Expr..Expr or Var -> List",
                                Some(it.loc),
                                ErrorType::Parsing,
                            );
                            Node {
                                tree: Tree::Empty(),
                                loc: it.loc,
                            }
                        }
                    }
                }
                TokenType::Fn => {
                    let name = if let Some(TokenType::Ident(name)) =
                        iter.peek().map(|t| &t.token)
                    {
                        self.prev_token = (*iter.peek().unwrap()).clone();
                        iter.next();
                        Some(intern(name))
                    } else {
                        None
                    };
                    let args = self.parse_args(iter);
                    let mut body = vec![];
                    if self.expect_token(iter, TokenType::FatArrow).is_some() {
                        body = self.parse_arrow_fn_body(iter);
                    }
                    return Node {
                        tree: Tree::Fn {
                            name,
                            args,
                            body,
                        },
                        loc: it.loc,
                    };
                }

                TokenType::Struct => {
                    if let Some(TokenType::Ident(name)) =
                        self.expect_token(iter, TokenType::Ident(String::new()))
                    {
                        if self.expect_token(iter, TokenType::OpenCurly).is_some() {
                            let (fields, methods) = self.parse_struct_body(iter);
                            return Node {
                                tree: Tree::StructDef {
                                    name: intern(&name),
                                    fields,
                                    methods,
                                },
                                loc: it.loc,
                            };
                        }
                    } else {
                        Logger::error("Expected Struct Name", Some(it.loc), ErrorType::Parsing);
                    }
                    Node {
                        tree: Tree::Empty(),
                        loc: it.loc,
                    }
                }

                TokenType::Match => {
                    let expr = Box::new(self.parse_expression(iter));

                    if self.expect_token(iter, TokenType::OpenCurly).is_none() {
                        return Node {
                            tree: Tree::Empty(),
                            loc: it.loc,
                        };
                    }

                    let mut arms: Vec<(Vec<Node>, Vec<Node>)> = vec![];
                    let mut els: Vec<Node> = vec![];

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
                        body = self.parse_arrow_fn_body(iter);
                        if patterns
                            .iter()
                            .any(|p| matches!(&p.tree, Tree::Ident(id) if *id == intern("_")))
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
                    Node {
                        tree: Tree::Match { expr, arms, els },
                        loc: it.loc,
                    }
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
                    Node {
                        tree: Tree::Import { path, alias },
                        loc: it.loc,
                    }
                }

                TokenType::Els | TokenType::ElsIf => {
                    Logger::error(
                        "Expected If statement first",
                        Some(it.loc),
                        ErrorType::Parsing,
                    );
                    Node {
                        tree: Tree::Empty(),
                        loc: it.loc,
                    }
                }
                _ => {
                    Logger::error(
                        &format!("Invalid Token {:?}", it.token),
                        Some(it.loc),
                        ErrorType::Parsing,
                    );
                    Node {
                        tree: Tree::Empty(),
                        loc: it.loc,
                    }
                }
            }
        } else {
            Logger::error(
                "Expected Statement",
                Some(self.prev_token.loc),
                ErrorType::Parsing,
            );
            Node {
                tree: Tree::Empty(),
                loc: self.prev_token.loc,
            }
        }
    }
}

