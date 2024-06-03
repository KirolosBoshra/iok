#[derive(Debug, Clone)]
pub enum TokenType {
    Number(usize),
    String(String),
    Plus,
    DPlue,
    Minus,
    DMinus,
    Multiply,
    Divide,
    Equal,
    EquEqu,
    Bang,
    NotEqu,
    Greater,
    Less,
    GreatEqu,
    LessEqu,
    OpenParen,
    CloseParen,
    OpenCurly,
    CloseCurly,
    Comma,
    Semi,
    Dot,
    DDot,
    ThinArrow,
    Let,
    Exit,
    Ident(String),
    If,
    Els,
    ElsIf,
    While,
    For,
    Empty,
}

// struct Loc {
//     x: usize,
//     y: usize,
// }

// $$ TODO : add loc to the token

#[derive(Debug, Clone)]
pub struct Lexer {
    input: String,
}
impl Lexer {
    pub fn new(input: String) -> Lexer {
        Lexer { input }
    }

    pub fn tokenize(self) -> Vec<TokenType> {
        let mut tokens = Vec::new();
        let mut iter = self.input.chars().peekable();

        while let Some(&c) = iter.peek() {
            match c {
                'a'..='z' | '_' | 'A'..='Z' => {
                    let mut buf = String::new();
                    while let Some(&c) = iter.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            buf.push(c);
                            iter.next();
                        } else {
                            break;
                        }
                    }
                    match buf.as_str() {
                        "exit" => tokens.push(TokenType::Exit),
                        "let" => tokens.push(TokenType::Let),
                        "if" => tokens.push(TokenType::If),
                        "els" => tokens.push(TokenType::Els),
                        "elsif" => tokens.push(TokenType::ElsIf),
                        "while" => tokens.push(TokenType::While),
                        "for" => tokens.push(TokenType::For),
                        _ => tokens.push(TokenType::Ident(buf)),
                    }
                }
                '0'..='9' => {
                    let mut number = String::new();
                    while let Some(&c) = iter.peek() {
                        if c.is_digit(10) {
                            number.push(c);
                            iter.next();
                        } else {
                            break;
                        }
                    }
                    let num = number.parse().unwrap();
                    tokens.push(TokenType::Number(num));
                }
                '\"' => {
                    iter.next();
                    let mut string = String::new();
                    while let Some(&c) = iter.peek() {
                        match c {
                            '\"' => {
                                iter.next();
                                break;
                            }
                            '\\' => {
                                string.push(c);
                                iter.next();
                                if *iter.peek().unwrap() == '\"' {
                                    string.push('\"');
                                    iter.next();
                                }
                            }
                            _ => {
                                string.push(c);
                                iter.next();
                            }
                        }
                    }
                    tokens.push(TokenType::String(string));
                }
                '(' => {
                    tokens.push(TokenType::OpenParen);
                    iter.next();
                }
                ')' => {
                    tokens.push(TokenType::CloseParen);
                    iter.next();
                }
                '{' => {
                    tokens.push(TokenType::OpenCurly);
                    iter.next();
                }
                '}' => {
                    tokens.push(TokenType::CloseCurly);
                    iter.next();
                }
                '+' => {
                    iter.next();
                    if *iter.peek().unwrap() == '+' {
                        iter.next();
                        tokens.push(TokenType::DPlue);
                    } else {
                        tokens.push(TokenType::Plus);
                    }
                }
                '-' => {
                    iter.next();
                    match *iter.peek().unwrap() {
                        '-' => {
                            iter.next();
                            tokens.push(TokenType::DMinus);
                        }
                        '>' => {
                            iter.next();
                            tokens.push(TokenType::ThinArrow);
                        }
                        _ => tokens.push(TokenType::Minus),
                    }
                }
                '*' => {
                    tokens.push(TokenType::Multiply);
                    iter.next();
                }
                '/' => {
                    iter.next();
                    if *iter.peek().unwrap() == '/' {
                        while *iter.peek().unwrap() != '\n' {
                            iter.next();
                        }
                    } else {
                        tokens.push(TokenType::Divide);
                    }
                }
                '=' => {
                    iter.next();
                    if *iter.peek().unwrap() == '=' {
                        tokens.push(TokenType::EquEqu);
                        iter.next();
                    } else {
                        tokens.push(TokenType::Equal);
                    }
                }
                '!' => {
                    iter.next();
                    if *iter.peek().unwrap() == '=' {
                        tokens.push(TokenType::NotEqu);
                        iter.next();
                    } else {
                        tokens.push(TokenType::Bang);
                    }
                }
                '>' => {
                    iter.next();
                    if *iter.peek().unwrap() == '=' {
                        tokens.push(TokenType::GreatEqu);
                        iter.next();
                    } else {
                        tokens.push(TokenType::Greater);
                    }
                }

                '<' => {
                    iter.next();
                    if *iter.peek().unwrap() == '=' {
                        tokens.push(TokenType::LessEqu);
                        iter.next();
                    } else {
                        tokens.push(TokenType::Less);
                    }
                }
                '.' => {
                    iter.next();
                    if *iter.peek().unwrap() == '.' {
                        tokens.push(TokenType::DDot);
                        iter.next();
                    } else {
                        tokens.push(TokenType::Dot);
                    }
                }
                ',' => {
                    iter.next();
                    tokens.push(TokenType::Comma);
                }
                ';' => {
                    tokens.push(TokenType::Semi);
                    iter.next();
                }
                _ => {
                    iter.next();
                }
            }
        }

        tokens
    }
}
