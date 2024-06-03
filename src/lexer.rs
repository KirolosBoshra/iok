#[derive(Debug, Clone)]
pub enum Token {
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

    pub fn tokenize(self) -> Vec<Token> {
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
                        "exit" => tokens.push(Token::Exit),
                        "let" => tokens.push(Token::Let),
                        "if" => tokens.push(Token::If),
                        "els" => tokens.push(Token::Els),
                        "elsif" => tokens.push(Token::ElsIf),
                        "while" => tokens.push(Token::While),
                        "for" => tokens.push(Token::For),
                        _ => tokens.push(Token::Ident(buf)),
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
                    tokens.push(Token::Number(num));
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
                    tokens.push(Token::String(string));
                }
                '(' => {
                    tokens.push(Token::OpenParen);
                    iter.next();
                }
                ')' => {
                    tokens.push(Token::CloseParen);
                    iter.next();
                }
                '{' => {
                    tokens.push(Token::OpenCurly);
                    iter.next();
                }
                '}' => {
                    tokens.push(Token::CloseCurly);
                    iter.next();
                }
                '+' => {
                    iter.next();
                    if *iter.peek().unwrap() == '+' {
                        iter.next();
                        tokens.push(Token::DPlue);
                    } else {
                        tokens.push(Token::Plus);
                    }
                }
                '-' => {
                    iter.next();
                    match *iter.peek().unwrap() {
                        '-' => {
                            iter.next();
                            tokens.push(Token::DMinus);
                        }
                        '>' => {
                            iter.next();
                            tokens.push(Token::ThinArrow);
                        }
                        _ => tokens.push(Token::Minus),
                    }
                }
                '*' => {
                    tokens.push(Token::Multiply);
                    iter.next();
                }
                '/' => {
                    iter.next();
                    if *iter.peek().unwrap() == '/' {
                        while *iter.peek().unwrap() != '\n' {
                            iter.next();
                        }
                    } else {
                        tokens.push(Token::Divide);
                    }
                }
                '=' => {
                    iter.next();
                    if *iter.peek().unwrap() == '=' {
                        tokens.push(Token::EquEqu);
                        iter.next();
                    } else {
                        tokens.push(Token::Equal);
                    }
                }
                '!' => {
                    iter.next();
                    if *iter.peek().unwrap() == '=' {
                        tokens.push(Token::NotEqu);
                        iter.next();
                    } else {
                        tokens.push(Token::Bang);
                    }
                }
                '>' => {
                    iter.next();
                    if *iter.peek().unwrap() == '=' {
                        tokens.push(Token::GreatEqu);
                        iter.next();
                    } else {
                        tokens.push(Token::Greater);
                    }
                }

                '<' => {
                    iter.next();
                    if *iter.peek().unwrap() == '=' {
                        tokens.push(Token::LessEqu);
                        iter.next();
                    } else {
                        tokens.push(Token::Less);
                    }
                }
                '.' => {
                    iter.next();
                    if *iter.peek().unwrap() == '.' {
                        tokens.push(Token::DDot);
                        iter.next();
                    } else {
                        tokens.push(Token::Dot);
                    }
                }
                ',' => {
                    iter.next();
                    tokens.push(Token::Comma);
                }
                ';' => {
                    tokens.push(Token::Semi);
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
