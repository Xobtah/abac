use std::str::FromStr;
use serde::{Deserialize, Serialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Rule {
    #[serde(rename = "String")]
    String(String),
    #[serde(rename = "Bool")]
    Bool(bool),
    #[serde(rename = "Integer")]
    Integer(i32),
    #[serde(rename = "Float")]
    Float(f32),
    #[serde(rename = "If")]
    If(String),
    #[serde(rename = "Eq")]
    Eq(String),
    #[serde(rename = "Set")]
    Set(String),
    #[serde(rename = "Tuple")]
    Tuple(Vec<Rule>),
}   

pub type Context = Vec<(String, Rule)>;

impl FromStr for Rule {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(parse_rule(s))
    }
}

fn parse_rule(rule: &str) -> Rule {
    let mut stack = vec![Rule::Tuple(Vec::new())];
    let mut buffer = String::new();
    let flush_buffer = |buffer: &mut String, stack: &mut Vec<Rule>| {
        if !buffer.is_empty() {
            let mut node: Rule;
            if buffer.parse::<f32>().is_ok() {
                node = Rule::Float(buffer.parse::<f32>().unwrap());
            } else if buffer.parse::<i32>().is_ok() {
                node = Rule::Integer(buffer.parse::<i32>().unwrap());
            } else if buffer.parse::<bool>().is_ok() {
                node = Rule::Bool(buffer.parse::<bool>().unwrap());
            } else {
                match buffer.as_str() {
                    "if" => {
                        node = Rule::If(buffer.clone());
                    }
                    "eq" => {
                        node = Rule::Eq(buffer.clone());
                    }
                    "set" => {
                        node = Rule::Set(buffer.clone());
                    }
                    _ => {
                        node = Rule::String(buffer.clone());
                    }
                }
            }
            let mut parent = stack.pop().unwrap();
            if let Rule::Tuple(ref mut children) = parent {
                children.push(node);
            }
            buffer.clear();
            stack.push(parent);
        }
    };
    for c in rule.chars() {
        if c == '(' {
            flush_buffer(&mut buffer, &mut stack);
            let node = Rule::Tuple(Vec::new());
            stack.push(node);
        } else if c == ')' {
            flush_buffer(&mut buffer, &mut stack);
            let node = stack.pop().unwrap();
            let mut parent = stack.pop().unwrap();
            if let Rule::Tuple(ref mut children) = parent {
                children.push(node);
            }
            stack.push(parent);
        } else if c == ' ' {
            flush_buffer(&mut buffer, &mut stack);
        } else {
            buffer.push(c);
        }
    }
    if let Rule::Tuple(ref mut children) = stack.pop().unwrap() {
        return children.pop().unwrap();
    } else {
        panic!("Invalid rule");
    }
}

impl Rule {
    pub fn eval(&self, context: &Context) -> Rule {
        match self {
            Rule::Tuple(children) => {
                match children.first() {
                    Some(Rule::If(_)) => {
                        assert_eq!(children.len(), 4);
                        let condition = children.get(1).unwrap().eval(context);
                        let then = children.get(2).unwrap().eval(context);
                        let otherwise = children.get(3).unwrap().eval(context);
                        match condition {
                            Rule::Bool(false) => otherwise,
                            Rule::Bool(true) => then,
                            _ => panic!("Invalid condition"),
                        }
                    }
                    Some(Rule::Eq(_)) => {
                        assert_eq!(children.len(), 3);
                        let left = children.get(1).unwrap().eval(context);
                        let right = children.get(2).unwrap().eval(context);
                        match (left, right) {
                            (Rule::String(l), Rule::String(r)) => Rule::Bool(l == r),
                            (Rule::Integer(l), Rule::Integer(r)) => Rule::Bool(l == r),
                            (Rule::Float(l), Rule::Float(r)) => Rule::Bool(l == r),
                            (Rule::Bool(l), Rule::Bool(r)) => Rule::Bool(l == r),
                            _ => panic!("Invalid comparison"),
                        }
                    }
                    Some(Rule::Set(_)) => {
                        Rule::Tuple(children.iter().skip(1).map(|child| child.eval(context)).collect())
                    }
                    _ => Rule::Tuple(vec![])
                }
            }
            Rule::String(val) => {
                if val.starts_with("$") {
                    let key = val.trim_start_matches("$");
                    match context.iter().find(|(k, _)| k == key) {
                        Some((_, val)) => val.clone(),
                        None => Rule::String(String::new()),
                    }
                } else {
                    Rule::String(val.clone())
                }
            }
            val => {val.clone()}
        }
    }
}
