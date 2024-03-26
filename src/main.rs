#[derive(Debug, Clone)]
enum LispExp {
    Symbol(String),
    Number(f64),
    List(Vec<LispExp>),
}

use std::fmt::Display;
impl Display for LispExp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            LispExp::Symbol(symb) => write!(f, "\"{symb}\""),
            LispExp::Number(num) => write!(f, "{}", num),
            LispExp::List(cdr) => {
                let cont: Vec<String> = cdr.iter().map(LispExp::to_string).collect();
                write!(f, "( {} )", cont.join(" "))
            }
        }
    }
}

#[derive(Debug)]
struct ListError(String);
impl Display for ListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Lisp Processing Error: {}", self.0)
    }
}

use std::convert::From;
impl From<String> for ListError {
    fn from(value: String) -> ListError {
        ListError(value)
    }
}

impl From<f64> for LispExp {
    fn from(value: f64) -> LispExp {
        LispExp::Number(value)
    }
}
impl From<&str> for LispExp {
    fn from(value: &str) -> LispExp {
        LispExp::Symbol(value.to_owned())
    }
}

enum Parser {
    OnSymbol,
    OnString { on_special: bool },
}

fn tokens(content: String) -> Result<Vec<String>, ListError> {
    let mut ret: Vec<String> = vec![];
    let mut buffer = String::new();
    let mut parser = Parser::OnSymbol;

    for chr in content.chars() {
        match parser {
            Parser::OnSymbol => match chr {
                '(' => {
                    ret.push(buffer);
                    ret.push("(".to_owned());
                    buffer = String::new();
                }
                ')' => {
                    ret.push(buffer);
                    ret.push(")".to_owned());
                    buffer = String::new();
                }
                ' ' => {
                    ret.push(buffer);
                    buffer = String::new();
                }
                '"' => {
                    parser = Parser::OnString { on_special: false };
                }
                other => {
                    if !other.is_whitespace() {
                        buffer.push_str(&other.to_string());
                    }
                }
            },
            Parser::OnString { on_special } => {
                if on_special {
                    let c = match chr {
                        '"' => Ok("\""),
                        '\\' => Ok("\\"),
                        'n' => Ok("\n"),
                        other => Err(format!("no special formatting for '\\{}'", other)),
                    }?;
                    buffer.push_str(c);
                    parser = Parser::OnString { on_special: false }
                } else {
                    match chr {
                        '\"' => {
                            ret.push(buffer);
                            buffer = String::new();
                            parser = Parser::OnSymbol;
                        }
                        '\\' => parser = Parser::OnString { on_special: true },
                        other => {
                            buffer.push_str(&other.to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(ret.into_iter().filter(|x| !x.is_empty()).collect())
}

fn parse(tokens: &[String]) -> Result<(LispExp, &[String]), ListError> {
    let (token, rest) = tokens
        .split_first()
        .ok_or(ListError("could not get token".to_string()))?;
    match &token[..] {
        "(" => read_seq(rest),
        ")" => Err(ListError("unexpected `)`".to_string())),
        _ => Ok((parse_atom(token), rest)),
    }
}

fn read_seq(tokens: &[String]) -> Result<(LispExp, &[String]), ListError> {
    let mut res: Vec<LispExp> = vec![];
    let mut xs = tokens;
    loop {
        let (next_token, rest) = xs
            .split_first()
            .ok_or(ListError("could not find closing `)`".to_string()))?;
        if next_token == ")" {
            return Ok((LispExp::List(res), rest));
        }
        let (exp, new_xs) = parse(xs)?;
        res.push(exp);
        xs = new_xs;
    }
}

fn parse_atom(token: &str) -> LispExp {
    token
        .parse::<f64>()
        .map(LispExp::from)
        .unwrap_or(LispExp::from(token))
}


fn main() {
    let content = std::fs::read_to_string("example.lsp").unwrap();
    let content = tokens(content).unwrap();
    let (parsed, missing) = parse(&content).map_err(|a| a.to_string()).unwrap();
    if !missing.is_empty() {
        println!("{missing:?}");
        panic!("not all tokens parsed")
    }
    println!("{parsed}");
}
