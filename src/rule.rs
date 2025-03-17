use serde::{de::value, Deserialize, Serialize};
use std::{f32::consts::E, str::FromStr};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    #[serde(rename = "And")]
    And(String),
    #[serde(rename = "Or")]
    Or(String),
    #[serde(rename = "Eq")]
    Eq(String),
    #[serde(rename = "Set")]
    Set(String),
    #[serde(rename = "Tuple")]
    Tuple(Vec<Rule>),
}

#[derive(Debug, PartialEq)]
pub enum Error {
    CannotParse(String),
    CannotParseAs(Rule, String),
    ConnotCompare(Rule, Rule),
    InvalidIfStatement(Rule),
    InvalidIfCondition(Rule),
    InvalidEqStatement(Rule),
}

pub type Context = Vec<(String, Rule)>;

impl FromStr for Rule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_rule(s)
    }
}

fn parse_rule(rule: &str) -> Result<Rule, Error> {
    let mut stack = vec![Rule::Tuple(Vec::new())];
    let mut buffer = String::new();
    let flush_buffer = |buffer: &mut String, stack: &mut Vec<Rule>| -> Result<(), Error> {
        if !buffer.is_empty() {
            let mut node: Rule;
            if buffer.parse::<i32>().is_ok() {
                node = Rule::Integer(
                    buffer
                        .parse::<i32>()
                        .ok()
                        .ok_or(Error::CannotParseAs(Rule::Integer(0), buffer.clone()))?,
                );
            } else if buffer.parse::<f32>().is_ok() {
                node = Rule::Float(
                    buffer
                        .parse::<f32>()
                        .ok()
                        .ok_or(Error::CannotParseAs(Rule::Float(0.0), buffer.clone()))?,
                );
            } else if buffer.parse::<bool>().is_ok() {
                node = Rule::Bool(
                    buffer
                        .parse::<bool>()
                        .ok()
                        .ok_or(Error::CannotParseAs(Rule::Bool(false), buffer.clone()))?,
                );
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
                    "and" => {
                        node = Rule::And(buffer.clone());
                    }
                    "or" => {
                        node = Rule::Or(buffer.clone());
                    }
                    _ => {
                        node = Rule::String(buffer.clone());
                    }
                }
            }
            let mut parent = stack.pop().ok_or(Error::CannotParse(String::from(rule)))?;
            if let Rule::Tuple(ref mut children) = parent {
                children.push(node);
            }
            buffer.clear();
            stack.push(parent);
        }
        Ok(())
    };
    for c in rule.chars() {
        if c == '(' {
            flush_buffer(&mut buffer, &mut stack)?;
            let node = Rule::Tuple(Vec::new());
            stack.push(node);
        } else if c == ')' {
            flush_buffer(&mut buffer, &mut stack)?;
            let node = stack.pop().ok_or(Error::CannotParse(String::from(rule)))?;
            let mut parent = stack.pop().ok_or(Error::CannotParse(String::from(rule)))?;
            if let Rule::Tuple(ref mut children) = parent {
                children.push(node);
            }
            stack.push(parent);
        } else if c == ' ' {
            flush_buffer(&mut buffer, &mut stack)?;
        } else {
            buffer.push(c);
        }
    }
    if let Rule::Tuple(ref mut children) =
        stack.pop().ok_or(Error::CannotParse(String::from(rule)))?
    {
        return children.pop().ok_or(Error::CannotParse(String::from(rule)));
    } else {
        return Err(Error::CannotParse(String::from(rule)));
    }
}

impl Rule {
    pub fn eval(&self, context: &Context) -> Result<Rule, Error> {
        match self {
            Rule::Tuple(children) => match children.first() {
                Some(Rule::If(_)) => {
                    if children.len() != 4 {
                        return Err(Error::InvalidIfStatement(self.clone()));
                    }
                    let condition = children
                        .get(1)
                        .ok_or(Error::InvalidIfCondition(self.clone()))?
                        .eval(context)?;
                    let then = children
                        .get(2)
                        .ok_or(Error::InvalidIfCondition(self.clone()))?
                        .eval(context)?;
                    let otherwise = children
                        .get(3)
                        .ok_or(Error::InvalidIfCondition(self.clone()))?
                        .eval(context)?;
                    match condition {
                        Rule::Bool(false) => Ok(otherwise),
                        Rule::Bool(true) => Ok(then),
                        _ => Err(Error::InvalidIfCondition(condition)),
                    }
                }
                Some(Rule::Eq(_)) => {
                    if children.len() != 3 {
                        return Err(Error::InvalidEqStatement(self.clone()));
                    }
                    let left = children
                        .get(1)
                        .ok_or(Error::InvalidEqStatement(self.clone()))?
                        .eval(context)?;
                    let right = children
                        .get(2)
                        .ok_or(Error::InvalidEqStatement(self.clone()))?
                        .eval(context)?;
                    match (left, right) {
                        (Rule::String(l), Rule::String(r)) => Ok(Rule::Bool(l == r)),
                        (Rule::Integer(l), Rule::Integer(r)) => Ok(Rule::Bool(l == r)),
                        (Rule::Float(l), Rule::Float(r)) => Ok(Rule::Bool(l == r)),
                        (Rule::Bool(l), Rule::Bool(r)) => Ok(Rule::Bool(l == r)),
                        (l, r) => Err(Error::ConnotCompare(l, r)),
                    }
                }
                Some(Rule::Set(_)) => Ok(Rule::Tuple(
                    children
                        .iter()
                        .skip(1)
                        .map(|child| child.eval(context))
                        .collect::<Result<Vec<Rule>, Error>>()?,
                )),
                Some(Rule::And(_)) => {
                    if children.len() != 3 {
                        return Err(Error::InvalidEqStatement(self.clone()));
                    }
                    let left = children
                        .get(1)
                        .ok_or(Error::InvalidEqStatement(self.clone()))?
                        .eval(context)?;
                    let right = children
                        .get(2)
                        .ok_or(Error::InvalidEqStatement(self.clone()))?
                        .eval(context)?;
                    match (left, right) {
                        (Rule::Bool(l), Rule::Bool(r)) => Ok(Rule::Bool(l && r)),
                        (l, r) => Err(Error::ConnotCompare(l, r)),
                    }
                }
                Some(Rule::Or(_)) => {
                    if children.len() != 3 {
                        return Err(Error::InvalidEqStatement(self.clone()));
                    }
                    let left = children
                        .get(1)
                        .ok_or(Error::InvalidEqStatement(self.clone()))?
                        .eval(context)?;
                    let right = children
                        .get(2)
                        .ok_or(Error::InvalidEqStatement(self.clone()))?
                        .eval(context)?;
                    match (left, right) {
                        (Rule::Bool(l), Rule::Bool(r)) => Ok(Rule::Bool(l || r)),
                        (l, r) => Err(Error::ConnotCompare(l, r)),
                    }
                }
                _ => Ok(Rule::Tuple(vec![])),
            },
            Rule::String(val) => {
                if val.starts_with("$") {
                    let key = val.trim_start_matches("$");
                    match context.iter().find(|(k, _)| k == key) {
                        Some((_, val)) => Ok(val.clone()),
                        None => Ok(Rule::String(String::new())),
                    }
                } else {
                    Ok(Rule::String(val.clone()))
                }
            }
            val => Ok(val.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rule() {
        assert_eq!(
            Rule::from_str(""),
            Err(Error::CannotParse(String::from("")))
        );
        assert_eq!(Rule::from_str("()"), Ok(Rule::Tuple(vec![])));
        assert_eq!(
            Rule::from_str("(if)"),
            Ok(Rule::Tuple(vec![Rule::If(String::from("if")),]))
        );
        assert_eq!(
            Rule::from_str("(if (eq )) (set create) (set))"),
            Err(Error::CannotParse(
                "(if (eq )) (set create) (set))".to_string()
            ))
        );
        assert_eq!(
            Rule::from_str("(or true true)"),
            Ok(Rule::Tuple(vec![
                Rule::Or(String::from("or")),
                Rule::Bool(true),
                Rule::Bool(true),
            ]))
        );
        assert_eq!(
            Rule::from_str("(and true true)"),
            Ok(Rule::Tuple(vec![
                Rule::And(String::from("and")),
                Rule::Bool(true),
                Rule::Bool(true),
            ]))
        );
        assert_eq!(
            Rule::from_str("(if (eq $name true) (set create) (set))"),
            Ok(Rule::Tuple(vec![
                Rule::If(String::from("if")),
                Rule::Tuple(vec![
                    Rule::Eq(String::from("eq")),
                    Rule::String(String::from("$name")),
                    Rule::Bool(true),
                ]),
                Rule::Tuple(vec![
                    Rule::Set(String::from("set")),
                    Rule::String(String::from("create")),
                ]),
                Rule::Tuple(vec![Rule::Set(String::from("set")),]),
            ]))
        );
        assert_eq!(
            Rule::from_str("(if (eq $name true) (set create) (set))"),
            Ok(Rule::Tuple(vec![
                Rule::If(String::from("if")),
                Rule::Tuple(vec![
                    Rule::Eq(String::from("eq")),
                    Rule::String(String::from("$name")),
                    Rule::Bool(true),
                ]),
                Rule::Tuple(vec![
                    Rule::Set(String::from("set")),
                    Rule::String(String::from("create")),
                ]),
                Rule::Tuple(vec![Rule::Set(String::from("set")),]),
            ]))
        );
    }

    #[test]
    fn test_eval_rule_and() {
        assert_eq!(
            Rule::from_str("(and true true)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(and true false)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(and false true)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(and false false)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(false))
        );
    }

    #[test]
    fn test_eval_rule_or() {
        assert_eq!(
            Rule::from_str("(or true true)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(or true false)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(or false true)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(or false false)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(false))
        );
    }

    #[test]
    fn test_eval_rule_eq() {
        assert_eq!(
            Rule::from_str("(eq john john)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq john jane)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(eq 10 10)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq 10 20)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(eq 10.0 10.0)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq 10.0 20.0)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(eq true true)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq true false)").unwrap().eval(&vec![]),
            Ok(Rule::Bool(false))
        );
    }

    #[test]
    fn test_eval_rule_if() {
        assert_eq!(
            Rule::from_str("(if true true false").unwrap().eval(&vec![]),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(if false true 10)").unwrap().eval(&vec![]),
            Ok(Rule::Integer(10))
        );
    }

    #[test]
    fn test_eval_rule() {
        assert_eq!(
            Rule::from_str("(if (eq $name john) (set create) (set))")
                .unwrap()
                .eval(&vec![
                    (String::from("name"), Rule::String(String::from("john"))),
                    (String::from("role"), Rule::String(String::from("admin"))),
                ]),
            Ok(Rule::Tuple(vec![Rule::String(String::from("create")),]))
        );
        assert_eq!(
            Rule::from_str("(if (eq $role admin) (set create read update delete list) (if (eq $role user) (set read list) (set)))")
                .unwrap()
                .eval(&vec![
                    (String::from("name"), Rule::String(String::from("john"))),
                    (String::from("role"), Rule::String(String::from("admin"))),
                ]),
            Ok(Rule::Tuple(vec![
                Rule::String(String::from("create")),
                Rule::String(String::from("read")),
                Rule::String(String::from("update")),
                Rule::String(String::from("delete")),
                Rule::String(String::from("list")),
            ]))
        );
        assert_eq!(
            Rule::from_str("(if (eq $role admin) (set create read update delete list) (if (eq $role user) (set read list) (set)))")
                .unwrap()
                .eval(&vec![
                    (String::from("name"), Rule::String(String::from("john"))),
                    (String::from("role"), Rule::String(String::from("user"))),
                ]),
            Ok(Rule::Tuple(vec![
                Rule::String(String::from("read")),
                Rule::String(String::from("list")),
            ]))
        );
    }
}
