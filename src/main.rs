#[derive(Debug, Clone)]
enum LispExp {
    Symbol(String),
    Number(f64),
    List(Vec<LispExp>),
}

impl LispExp {
    fn name(&self) -> &'static str {
        match self {
            LispExp::Number(_)=>"Number",
            LispExp::Symbol(_)=>"Symbol",
            LispExp::List(_)=>"List",
        }
    }
    fn get_symbol(&self) -> Result<&str, ListError> {
        if let LispExp::Symbol(n) = self {
            Ok(n)
        } else {
            Err(format!("`{self}`\n{self:?}\nis not a symbol, it's a {}", self.name()).into())
        }
    }
    fn get_number(&self) -> Result<f64, ListError> {
        if let LispExp::Number(n) = self {
            Ok(*n)
        } else {
            Err(ListError(format!("{self:?} is not a number")))
        }
    }
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
impl From<&str> for ListError {
    fn from(value: &str) -> ListError {
        ListError(value.to_owned())
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
impl From<String> for LispExp {
    fn from(value: String) -> LispExp {
        LispExp::Symbol(value)
    }
}
impl From<Vec<LispExp>> for LispExp {
    fn from(value: Vec<LispExp>) -> LispExp {
        LispExp::List(value)
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
                ' '|'\n'|'\t' => {
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
        .ok_or(ListError::from("could not get token"))?;
    match &token[..] {
        "(" => read_seq(rest),
        ")" => Err(ListError::from("unexpected `)`")),
        _ => Ok((parse_atom(token), rest)),
    }
}

fn read_seq(tokens: &[String]) -> Result<(LispExp, &[String]), ListError> {
    let mut res: Vec<LispExp> = vec![];
    let mut xs = tokens;
    loop {
        let (next_token, rest) = xs
            .split_first()
            .ok_or(ListError::from("could not find closing `)`"))?;
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

use std::boxed::Box;
use std::collections::HashMap;
type LispFN = Box<dyn Fn(&LispInfo, &[LispExp]) -> Result<LispExp, ListError>>;
struct LispInfo {
    functions: HashMap<String, LispFN>,
    root: LispExp,
}

impl LispInfo {

    fn value(&self, vl: &LispExp) -> Result<LispExp, ListError> {
        if let LispExp::List(stuff) = vl {
            let (car, cdr) = stuff.split_first()
                .ok_or(ListError::from("could not get token"))?;
            let car_str = car.get_symbol()?;
            if self.functions.contains_key(car_str) {
                Ok(self.exec(car_str, cdr)?.clone())
            } else if !cdr.is_empty() {
                Err(ListError(format!("symbol {} not defined as funtion so it takes arguments", car)))
            } else {
                Ok(car.clone())
            }
        } else {
            Ok(vl.clone())
        }
    }
    fn exec(&self, car: &str, cdr: &[LispExp]) -> Result<LispExp, ListError> {
        let func = self.functions
            .get(car)
            .ok_or(ListError(format!("can't find function {car}")))?;
        //func(&cdr.iter().map(|a|self.value(a)).collect::<Result<Vec<LispExp>, ListError>>()?)
        func(self, cdr)
    }
    fn run(&self) -> Result<LispExp, ListError> {
        self.value(&self.root)
    }
}

macro_rules! record {
  ($env: expr, $symb: expr, $check_fn:expr) => {{
      $env.insert(String::from($symb), Box::new($check_fn))
  }}
}

// helper functions
fn get_floats(cont: &[LispExp]) -> Result<Vec<f64>, ListError> {
    cont
        .iter()
        .map(LispExp::get_number)
        .collect()
}

fn unpack(cont: &[LispExp]) -> Result<(&LispExp, &[LispExp]), ListError> {
    cont
        .split_first()
        .ok_or(ListError::from("could not get token"))
}
//fn car(cont: &[LispExp]) -> Result<&LispExp, ListError> {
//    cont
//        .first()
//        .ok_or(ListError::from("could not get token"))
//}
//fn cdr(cont: &[LispExp]) -> Result<&[LispExp], ListError> {
//    let (_, cdr) = cont
//        .split_first()
//        .ok_or(ListError::from("could not get token"))?;
//    Ok(cdr)
//}

// after implementing user func definitions i could
// implement eval_some and only eval lists with car Symb('~') or smth like that
// macro creation would be as simple as:
/*
( def is-three
    ' (y) (
        (= ~(y) 3)
    )
)
*/
fn eval_all(env: &LispInfo, r: &[LispExp]) -> Result<Vec<LispExp>, ListError> {
    r.iter().map(|a|env.value(a)).collect()
}

fn lisp_add(env: &LispInfo, cont: &[LispExp]) -> Result<LispExp, ListError> {
    let cont = eval_all(env, cont)?;
    let (car, cdr) = unpack(&cont)?;
    let car = car.get_number()?;
    Ok(get_floats(cdr)?.iter().fold(car, |acc, f|acc+f).into())
}
fn lisp_sub(env: &LispInfo, cont: &[LispExp]) -> Result<LispExp, ListError> {
    let cont = eval_all(env, cont)?;
    let (car, cdr) = unpack(&cont)?;
    let car = car.get_number()?;
    Ok(get_floats(cdr)?.iter().fold(car, |acc, f|acc-f).into())
}
fn lisp_mul(env: &LispInfo, cont: &[LispExp]) -> Result<LispExp, ListError> {
    let cont = eval_all(env, cont)?;
    let (car, cdr) = unpack(&cont)?;
    let car = car.get_number()?;
    Ok(get_floats(cdr)?.iter().fold(car, |acc, f|acc+f).into())
}
fn lisp_div(env: &LispInfo, cont: &[LispExp]) -> Result<LispExp, ListError> {
    let cont = eval_all(env, cont)?;
    let (car, cdr) = unpack(&cont)?;
    let car = car.get_number()?;
    Ok(get_floats(cdr)?.iter().fold(car, |acc, f|acc+f).into())
}

fn lisp_debug(_env: &LispInfo, cont: &[LispExp]) -> Result<LispExp, ListError> {
    for item in cont {
        println!("{item}");
    }
    Ok((0.0).into())
}

fn lisp_print(env: &LispInfo, cont: &[LispExp]) -> Result<LispExp, ListError> {
    let cont = eval_all(env, cont)?;
    for item in cont {
        println!("{item}");
    }
    Ok((0.0).into())
}

fn lisp_also(env: &LispInfo, cont: &[LispExp]) -> Result<LispExp, ListError> {
    let ev = eval_all(env, cont)?;
    let ev = ev.last().ok_or(ListError::from(""))?;
    Ok(ev.clone())
}

fn builtin_funcs() -> HashMap<String, LispFN> {
    let mut funcs: HashMap<String, LispFN> = HashMap::new();
    record!(funcs, "+", lisp_add);
    record!(funcs, "-", lisp_sub);
    record!(funcs, "*", lisp_mul);
    record!(funcs, "/", lisp_div);
    record!(funcs, "print", lisp_print);
    record!(funcs, "'", lisp_debug);
    record!(funcs, ",", lisp_also);
    funcs
}

fn main() {
    let content = std::fs::read_to_string("example.lsp").unwrap();
    let content = tokens(content).unwrap();
    let (parsed, missing) = parse(&content).map_err(|a| a.to_string()).unwrap();
    if !missing.is_empty() {
        println!("{missing:?}");
        panic!("not all tokens parsed")
    }
    let lisp = LispInfo {
        root: parsed,
        functions: builtin_funcs(),
    };
    let code = lisp.run();
    let code = code.map(|a|a.get_number());
    let code = code.expect("failed to run code");
    let code = code.expect("code didn't exit with number");
    let code = unsafe { code.to_int_unchecked::<i32>() };
    std::process::exit(code);
}

