#[derive(Debug, PartialEq)]
pub enum Tk {
    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LSquare,
    RSquare,

    // Simple
    Comma,
    Dot,
    Colon,
    Semicolon,

    // Whitespace
    NewLine,

    // Literals
    LitFloat(f64),
    LitInt(isize),
    LitStr(String),

    // Identifiers
    Name(String),
    NameInfix(String),

    // Special
    Eof,
    Error(String),
}

// Helpers
#[inline]
fn is_special(c: char) -> bool {
    "!@$%^&*-+=|/<>".contains(c)
}

// Lexer
pub fn lex(source: String) -> Vec<Tk> {
    let mut ts: Vec<Tk> = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '(' => ts.push(Tk::LParen),
            ')' => ts.push(Tk::RParen),
            '{' => ts.push(Tk::LBrace),
            '}' => ts.push(Tk::RBrace),
            '[' => ts.push(Tk::LSquare),
            ']' => ts.push(Tk::RSquare),
            '.' => ts.push(Tk::Dot),
            ',' => ts.push(Tk::Comma),
            ':' => ts.push(Tk::Colon),
            ';' => ts.push(Tk::Semicolon),

            '0'..='9' => {
                // Parse Number
                let mut digits = String::from(c);
                let mut is_float = false;

                while let Some(c) = chars.peek() {
                    match c {
                        '0'..='9' => digits.push(chars.next().unwrap()),
                        '.' => {
                            is_float = true;
                            digits.push(chars.next().unwrap());
                        }
                        _ => break,
                    }
                }

                if is_float {
                    let f = digits.parse::<f64>().unwrap();
                    ts.push(Tk::LitFloat(f));
                } else {
                    let f = digits.parse::<isize>().unwrap();
                    ts.push(Tk::LitInt(f));
                }
            }

            '"' => {
                // Parse String
                let mut s = String::new();
                let mut ok = false;
                while let Some(c) = chars.peek() {
                    match c {
                        '"' => {
                            ok = true;
                            chars.next().unwrap();
                            ts.push(Tk::LitStr(s.clone()));
                            break;
                        }
                        _ => {
                            s.push(chars.next().unwrap()) // TODO: Handle NewLine
                        }
                    }
                }

                if !ok {
                    ts.push(Tk::Error("Unterminated String".to_string()))
                }
            }

            'a'..='z' | 'A'..='Z' | '_' => {
                // Parse Identifier
                let mut name = String::from(c);

                while let Some(c) = chars.peek() {
                    match c {
                        'a'..='z' | 'A'..='Z' | '_' | '0'..='9' => name.push(chars.next().unwrap()),
                        _ => {
                            ts.push(Tk::Name(name.clone()));
                            break;
                        }
                    }
                }

                if chars.peek() == None {
                    ts.push(Tk::Name(name.clone()));
                }
            }

            c if is_special(c.clone()) => {
                let mut name = String::from(c);
                while let Some(c) = chars.peek() {
                    match c {
                        x if is_special(x.clone()) => name.push(chars.next().unwrap()),
                        _ => {
                            ts.push(Tk::NameInfix(name.clone()));
                            break;
                        }
                    }
                }
                if chars.peek() == None {
                    ts.push(Tk::NameInfix(name.clone()));
                }
            }

            '\r' | '\t' | ' ' => (), // Ignore WhiteSpace

            '\n' => {
                // Handle NewLine
                ts.push(Tk::NewLine);
            }

            _ => {
                panic!("Unexpected Character")
            }
        }
    }
    ts.push(Tk::Eof);
    ts
}
