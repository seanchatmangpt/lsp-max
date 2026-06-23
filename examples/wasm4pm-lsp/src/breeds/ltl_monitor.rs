//! `ltl_monitor` — LTL Runtime Monitor breed.
//!
//! Finite-trace LTL semantics: Bauer et al., "Runtime Verification for LTL and TLTL", ACM TOSEM 2011.
//!
//! Family: FormalMethods / Paper: `bauer2011ltl` / Oracle value: 1.0 / Status: CANDIDATE

use crate::breeds::breed::{BreedInput, CognitiveBreed};
use serde_json::Value;

pub struct LtlMonitor;

#[derive(Debug, Clone)]
enum F {
    Atom(String),
    Not(Box<F>),
    And(Box<F>, Box<F>),
    Or(Box<F>, Box<F>),
    Next(Box<F>),
    G(Box<F>),
    Finally(Box<F>),
    Until(Box<F>, Box<F>),
}

struct Parser {
    tok: Vec<String>,
    pos: usize,
}

impl Parser {
    fn new(s: &str) -> Self {
        let expanded = s.replace('(', " ( ").replace(')', " ) ");
        Self {
            tok: expanded.split_whitespace().map(str::to_string).collect(),
            pos: 0,
        }
    }
    fn peek(&self) -> Option<&str> {
        self.tok.get(self.pos).map(String::as_str)
    }
    fn eat(&mut self) -> Option<String> {
        self.tok.get(self.pos).cloned().inspect(|_| {
            self.pos += 1;
        })
    }
    // Precedence (low→high): ->, U, or, and, unary, atom
    fn expr(&mut self) -> Option<F> {
        let l = self.until()?;
        if matches!(self.peek(), Some("->") | Some("→")) {
            self.eat();
            let r = self.until()?;
            Some(F::Or(Box::new(F::Not(Box::new(l))), Box::new(r)))
        } else {
            Some(l)
        }
    }
    fn until(&mut self) -> Option<F> {
        let l = self.or()?;
        if self.peek() == Some("U") {
            self.eat();
            let r = self.or()?;
            Some(F::Until(Box::new(l), Box::new(r)))
        } else {
            Some(l)
        }
    }
    fn or(&mut self) -> Option<F> {
        let l = self.and()?;
        if matches!(self.peek(), Some("or") | Some("||")) {
            self.eat();
            let r = self.and()?;
            Some(F::Or(Box::new(l), Box::new(r)))
        } else {
            Some(l)
        }
    }
    fn and(&mut self) -> Option<F> {
        let l = self.unary()?;
        if matches!(self.peek(), Some("and") | Some("&&")) {
            self.eat();
            let r = self.unary()?;
            Some(F::And(Box::new(l), Box::new(r)))
        } else {
            Some(l)
        }
    }
    fn unary(&mut self) -> Option<F> {
        match self.peek()? {
            "not" | "!" => {
                self.eat();
                Some(F::Not(Box::new(self.unary()?)))
            }
            "G" => {
                self.eat();
                Some(F::G(Box::new(self.unary()?)))
            }
            "F" => {
                self.eat();
                Some(F::Finally(Box::new(self.unary()?)))
            }
            "X" => {
                self.eat();
                Some(F::Next(Box::new(self.unary()?)))
            }
            _ => self.atom(),
        }
    }
    fn atom(&mut self) -> Option<F> {
        if self.peek() == Some("(") {
            self.eat();
            let inner = self.expr()?;
            if self.peek() == Some(")") {
                self.eat();
            }
            Some(inner)
        } else {
            self.eat().map(F::Atom)
        }
    }
}

// Returns Some(bool) if the verdict is determined on this finite prefix; None = undetermined.
fn eval(f: &F, tr: &[Value], i: usize) -> Option<bool> {
    let n = tr.len();
    match f {
        F::Atom(name) => {
            let v = tr.get(i)?.get(name.as_str())?;
            Some(
                v.as_bool()
                    .unwrap_or_else(|| v.as_f64().map(|x| x != 0.0).unwrap_or(false)),
            )
        }
        F::Not(x) => Some(!eval(x, tr, i)?),
        F::And(l, r) => Some(eval(l, tr, i)? && eval(r, tr, i)?),
        F::Or(l, r) => Some(eval(l, tr, i)? || eval(r, tr, i)?),
        F::Next(x) => {
            if i + 1 >= n {
                Some(false)
            } else {
                eval(x, tr, i + 1)
            }
        }
        F::G(x) => {
            for j in i..n {
                match eval(x, tr, j) {
                    Some(false) => return Some(false),
                    None => return None,
                    _ => {}
                }
            }
            Some(true)
        }
        F::Finally(x) => {
            for j in i..n {
                match eval(x, tr, j) {
                    Some(true) => return Some(true),
                    None => return None,
                    _ => {}
                }
            }
            Some(false)
        }
        F::Until(phi, psi) => {
            for j in i..n {
                match eval(psi, tr, j) {
                    Some(true) => return Some(true),
                    Some(false) => match eval(phi, tr, j) {
                        Some(false) => return Some(false),
                        None => return None,
                        _ => {}
                    },
                    None => return None,
                }
            }
            // ψ never witnessed; U is false on a closed finite trace.
            Some(false)
        }
    }
}

const DEFAULT_FORMULA: &str = "G ( request -> F response )";
const DEFAULT_TRACE: &str = r#"[
  {"request": true,  "response": false},
  {"request": false, "response": true},
  {"request": false, "response": false}
]"#;

impl CognitiveBreed for LtlMonitor {
    fn breed_id(&self) -> &'static str {
        "ltl_monitor"
    }

    fn run(&self, input: &BreedInput) -> Option<serde_json::Value> {
        let formula_str = input
            .get("formula")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_FORMULA);
        let trace: Vec<Value> = input
            .get("trace")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .or_else(|| serde_json::from_str(DEFAULT_TRACE).ok())?;

        let formula = Parser::new(formula_str).expr()?;
        let (verdict, value) = match eval(&formula, &trace, 0) {
            Some(true) => ("TRUE", 1.0_f64),
            Some(false) => ("FALSE", 0.0_f64),
            None => ("UNKNOWN", 0.5_f64),
        };
        Some(serde_json::json!({ "verdict": verdict, "value": value }))
    }
}
