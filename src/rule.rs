use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Debug, Serialize, Clone, PartialEq)]
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
    #[serde(rename = "In")]
    In(String),
    #[serde(rename = "List")]
    List(String),
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
    InvalidOrStatement(Rule),
    InvalidAndStatement(Rule),
    InvalidInStatement(Rule),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CannotParse(s) => write!(f, "Cannot parse '{}'", s),
            Error::CannotParseAs(r, s) => write!(f, "Cannot parse '{}' as {:?}", s, r),
            Error::ConnotCompare(l, r) => write!(f, "Cannot compare {:?} with {:?}", l, r),
            Error::InvalidIfStatement(r) => write!(f, "Invalid if statement {:?}", r),
            Error::InvalidIfCondition(r) => write!(f, "Invalid if condition {:?}", r),
            Error::InvalidEqStatement(r) => write!(f, "Invalid eq statement {:?}", r),
            Error::InvalidOrStatement(r) => write!(f, "Invalid or statement {:?}", r),
            Error::InvalidAndStatement(r) => write!(f, "Invalid and statement {:?}", r),
            Error::InvalidInStatement(r) => write!(f, "Invalid in statement {:?}", r),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Context(Vec<(String, Rule)>);

impl FromStr for Context {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut context = Context(Vec::new());
        for pair in s.split(',') {
            if pair.is_empty() {
                continue;
            }
            let mut iter = pair.split(':');
            let key = iter.next().ok_or(Error::CannotParse(String::from(s)))?;
            let value = iter.next().ok_or(Error::CannotParse(String::from(s)))?;
            context.0.push((String::from(key), Rule::from_literal(value)?));
        }
        Ok(context)
    }
}

impl FromStr for Rule {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_rule(s)
    }
}

impl<'a> Deserialize<'a> for Rule {
    fn deserialize<D>(deserializer: D) -> Result<Rule, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        let rule = Rule::from_str(s.as_str()).map_err(serde::de::Error::custom)?;
        rule.eval(&Context::from_str("").unwrap()).map_err(serde::de::Error::custom)?;
        Ok(rule)
    }
}

fn parse_rule(rule: &str) -> Result<Rule, Error> {
    let mut stack = vec![Rule::Tuple(Vec::new())];
    let mut buffer = String::new();
    let flush_buffer = |buffer: &mut String, stack: &mut Vec<Rule>| -> Result<(), Error> {
        if !buffer.is_empty() {
            let mut node: Rule;
            let mut parent = stack.pop().ok_or(Error::CannotParse(String::from(rule)))?;
            let mut children = match parent {
                Rule::Tuple(ref mut children) => children,
                _ => return Err(Error::CannotParse(String::from(rule))),
            };
            if buffer.parse::<i32>().is_ok() {
                node = Rule::from_literal(buffer.as_str())?;
            } else if buffer.parse::<f32>().is_ok() {
                node = Rule::from_literal(buffer.as_str())?;
            } else if buffer.parse::<bool>().is_ok() {
                node = Rule::from_literal(buffer.as_str())?;
            } else if children.is_empty() {
                match buffer.as_str() {
                    "if" => {
                        node = Rule::If(buffer.clone());
                    }
                    "eq" => {
                        node = Rule::Eq(buffer.clone());
                    }
                    "list" => {
                        node = Rule::List(buffer.clone());
                    }
                    "and" => {
                        node = Rule::And(buffer.clone());
                    }
                    "or" => {
                        node = Rule::Or(buffer.clone());
                    }
                    "in" => {
                        node = Rule::In(buffer.clone());
                    }
                    _ => {
                        node = Rule::String(buffer.clone());
                    }
                }
            } else {
                node = Rule::String(buffer.clone());
            }
            children.push(node);
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
    pub fn from_literal(s: &str) -> Result<Rule, Error> {
        if s.parse::<i32>().is_ok() {
            Ok(Rule::Integer(s.parse::<i32>().ok().ok_or(Error::CannotParseAs(Rule::Integer(0), s.to_string()))?))
        } else if s.parse::<f32>().is_ok() {
            Ok(Rule::Float(s.parse::<f32>().ok().ok_or(Error::CannotParseAs(Rule::Float(0.0), s.to_string()))?))
        } else if s.parse::<bool>().is_ok() {
            Ok(Rule::Bool(s.parse::<bool>().ok().ok_or(Error::CannotParseAs(Rule::Bool(false), s.to_string()))?))
        } else {
            Ok(Rule::String(s.to_string()))
        }
    }

    pub fn eval(&self, context: &Context) -> Result<Rule, Error> {
        match self {
            Rule::Tuple(children) => match children.first() {
                Some(Rule::If(_)) => {
                    if children.len() != 4 {
                        return Err(Error::InvalidIfStatement(self.clone()));
                    }
                    let condition = children
                        .get(1)
                        .ok_or(Error::InvalidIfStatement(self.clone()))?
                        .eval(context)?;
                    let then = children
                        .get(2)
                        .ok_or(Error::InvalidIfStatement(self.clone()))?
                        .eval(context)?;
                    let otherwise = children
                        .get(3)
                        .ok_or(Error::InvalidIfStatement(self.clone()))?
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
                Some(Rule::List(_)) => Ok(Rule::Tuple(
                    children
                        .iter()
                        .skip(1)
                        .map(|child| child.eval(context))
                        .collect::<Result<Vec<Rule>, Error>>()?,
                )),
                Some(Rule::And(_)) => {
                    if children.len() != 3 {
                        return Err(Error::InvalidAndStatement(self.clone()));
                    }
                    let left = children
                        .get(1)
                        .ok_or(Error::InvalidAndStatement(self.clone()))?
                        .eval(context)?;
                    let right = children
                        .get(2)
                        .ok_or(Error::InvalidAndStatement(self.clone()))?
                        .eval(context)?;
                    match (left, right) {
                        (Rule::Bool(l), Rule::Bool(r)) => Ok(Rule::Bool(l && r)),
                        (l, r) => Err(Error::ConnotCompare(l, r)),
                    }
                }
                Some(Rule::Or(_)) => {
                    if children.len() != 3 {
                        return Err(Error::InvalidOrStatement(self.clone()));
                    }
                    let left = children
                        .get(1)
                        .ok_or(Error::InvalidOrStatement(self.clone()))?
                        .eval(context)?;
                    let right = children
                        .get(2)
                        .ok_or(Error::InvalidOrStatement(self.clone()))?
                        .eval(context)?;
                    match (left, right) {
                        (Rule::Bool(l), Rule::Bool(r)) => Ok(Rule::Bool(l || r)),
                        (l, r) => Err(Error::ConnotCompare(l, r)),
                    }
                }
                Some(Rule::In(_)) => {
                    if children.len() != 3 {
                        return Err(Error::InvalidInStatement(self.clone()));
                    }
                    let left = children
                        .get(1)
                        .ok_or(Error::InvalidInStatement(self.clone()))?
                        .eval(context)?;
                    let right = children
                        .get(2)
                        .ok_or(Error::InvalidInStatement(self.clone()))?
                        .eval(context)?;
                    match (left, right) {
                        (Rule::String(l), Rule::Tuple(ref r)) => Ok(Rule::Bool(r.contains(&Rule::String(l)))),
                        (Rule::Integer(l), Rule::Tuple(ref r)) => Ok(Rule::Bool(r.contains(&Rule::Integer(l)))),
                        (Rule::Float(l), Rule::Tuple(ref r)) => Ok(Rule::Bool(r.contains(&Rule::Float(l)))),
                        (Rule::Bool(l), Rule::Tuple(ref r)) => Ok(Rule::Bool(r.contains(&Rule::Bool(l)))),
                        (l, r) => Err(Error::InvalidInStatement(self.clone())),
                    }
                }
                _ => Ok(Rule::Tuple(vec![])),
            },
            Rule::String(val) => {
                if val.starts_with("$") {
                    let key = val.trim_start_matches("$");
                    match context.0.iter().find(|(k, _)| k == key) {
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
    fn test_parse_context_ok() {
        assert_eq!(
            Context::from_str("name:John,age:20,weight:70.5,active:true"),
            Ok(Context(vec![
                (String::from("name"), Rule::String(String::from("John"))),
                (String::from("age"), Rule::Integer(20)),
                (String::from("weight"), Rule::Float(70.5)),
                (String::from("active"), Rule::Bool(true)),
            ]))
        );
    }

    #[test]
    fn test_parse_literal_ok() {
        assert_eq!(Rule::from_literal("John"), Ok(Rule::String(String::from("John"))));
        assert_eq!(Rule::from_literal("20"), Ok(Rule::Integer(20)));
        assert_eq!(Rule::from_literal("70.5"), Ok(Rule::Float(70.5)));
        assert_eq!(Rule::from_literal("true"), Ok(Rule::Bool(true)));
        assert_eq!(Rule::from_literal(""), Ok(Rule::String(String::new())));
        assert_eq!(Rule::from_literal("0d!"), Ok(Rule::String(String::from("0d!"))));
    }

    #[test]
    fn test_parse_rule_ok() {
        assert_eq!(Rule::from_str("()"), Ok(Rule::Tuple(vec![])));
        assert_eq!(
            Rule::from_str("(if)"),
            Ok(Rule::Tuple(vec![Rule::If(String::from("if")),]))
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
            Rule::from_str("(if (eq $name true) (list create) (list))"),
            Ok(Rule::Tuple(vec![
                Rule::If(String::from("if")),
                Rule::Tuple(vec![
                    Rule::Eq(String::from("eq")),
                    Rule::String(String::from("$name")),
                    Rule::Bool(true),
                ]),
                Rule::Tuple(vec![
                    Rule::List(String::from("list")),
                    Rule::String(String::from("create")),
                ]),
                Rule::Tuple(vec![Rule::List(String::from("list")),]),
            ]))
        );
        assert_eq!(
            Rule::from_str("(if (eq $name true) (list create) (list))"),
            Ok(Rule::Tuple(vec![
                Rule::If(String::from("if")),
                Rule::Tuple(vec![
                    Rule::Eq(String::from("eq")),
                    Rule::String(String::from("$name")),
                    Rule::Bool(true),
                ]),
                Rule::Tuple(vec![
                    Rule::List(String::from("list")),
                    Rule::String(String::from("create")),
                ]),
                Rule::Tuple(vec![Rule::List(String::from("list")),]),
            ]))
        );
    }

    #[test]
    fn test_parse_rule_err() {
        assert_eq!(
            Rule::from_str(""),
            Err(Error::CannotParse(String::from("")))
        );
        assert_eq!(
            Rule::from_str("(if (eq )) (list create) (list))"),
            Err(Error::CannotParse(
                "(if (eq )) (list create) (list))".to_string()
            ))
        );
    }

    #[test]
    fn test_eval_rule_in_ok() {
        assert_eq!(
            Rule::from_str("(in john (list))").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(in 10 (list))").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(in john (list john jane))").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(in john (list jane))").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(in john (list 10))").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(in john (list 10 john))").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
    }
    #[test]
    fn test_eval_rule_in_err() {
        assert_eq!(
            Rule::from_str("(in john jane)").unwrap().eval(&Context::from_str("").unwrap()),
            Err(Error::InvalidInStatement(Rule::Tuple(vec![
                Rule::In(String::from("in")),
                Rule::String(String::from("john")),
                Rule::String(String::from("jane")),
            ])))
        );
        assert_eq!(
            Rule::from_str("(in (list john) jane)").unwrap().eval(&Context::from_str("").unwrap()),
            Err(Error::InvalidInStatement(Rule::Tuple(vec![
                Rule::In(String::from("in")),
                Rule::Tuple(vec![Rule::List(String::from("list")), Rule::String(String::from("john"))]),
                Rule::String(String::from("jane")),
            ])))
        );
    }

    #[test]
    fn test_eval_rule_and_ok() {
        assert_eq!(
            Rule::from_str("(and true true)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(and true false)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(and false true)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(and false false)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
    }

    #[test]
    fn test_eval_rule_and_err() {
        assert_eq!(
            Rule::from_str("(and true)").unwrap().eval(&Context::from_str("").unwrap()),
            Err(Error::InvalidAndStatement(Rule::Tuple(vec![
                Rule::And(String::from("and")),
                Rule::Bool(true),
            ])))
        );
    }

    #[test]
    fn test_eval_rule_or_ok() {
        assert_eq!(
            Rule::from_str("(or true true)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(or true false)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(or false true)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(or false false)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
    }

    #[test]
    fn test_eval_rule_or_err() {
        assert_eq!(
            Rule::from_str("(or true)").unwrap().eval(&Context::from_str("").unwrap()),
            Err(Error::InvalidOrStatement(Rule::Tuple(vec![
                Rule::Or(String::from("or")),
                Rule::Bool(true),
            ])))
        );
    }

    #[test]
    fn test_eval_rule_eq_ok() {
        assert_eq!(
            Rule::from_str("(eq $a $b)").unwrap
            ().eval(&Context::from_str("a:10,b:10").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq john john)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq john jane)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(eq 10 10)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq 10 20)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(eq 10.0 10.0)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq 10.0 20.0)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
        assert_eq!(
            Rule::from_str("(eq true true)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(eq true false)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(false))
        );
    }

    #[test]
    fn test_eval_rule_eq_err() {
        assert_eq!(
            Rule::from_str("(eq)").unwrap().eval(&Context::from_str("").unwrap()),
            Err(Error::InvalidEqStatement(Rule::Tuple(vec![
                Rule::Eq(String::from("eq")),
            ])))
        );
        assert_eq!(
            Rule::from_str("(eq john)").unwrap().eval(&Context::from_str("").unwrap()),
            Err(Error::InvalidEqStatement(Rule::Tuple(vec![
                Rule::Eq(String::from("eq")),
                Rule::String(String::from("john")),
            ]))
        ));
    }

    #[test]
    fn test_eval_rule_if_ok() {
        assert_eq!(
            Rule::from_str("(if true true false").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Bool(true))
        );
        assert_eq!(
            Rule::from_str("(if false true 10)").unwrap().eval(&Context::from_str("").unwrap()),
            Ok(Rule::Integer(10))
        );
    }

    #[test]
    fn test_eval_rule_if_err() {
        assert_eq!(
            Rule::from_str("(if)").unwrap().eval(&Context::from_str("").unwrap()),
            Err(Error::InvalidIfStatement(Rule::Tuple(vec![
                Rule::If(String::from("if")),
            ])))
        );
    }

    #[test]
    fn test_eval_rule_ok() {
        assert_eq!(
            Rule::from_str("(if (eq $name john) (list create) (list))")
                .unwrap()
                .eval(&Context::from_str("name:john,role:admin").unwrap()),
            Ok(Rule::Tuple(vec![Rule::String(String::from("create")),]))
        );
        assert_eq!(
            Rule::from_str("(if (eq $role admin) (list create read update delete list) (if (eq $role user) (list read list) (list)))")
                .unwrap()
                .eval(&Context::from_str("name:john,role:admin").unwrap()),
            Ok(Rule::Tuple(vec![
                Rule::String(String::from("create")),
                Rule::String(String::from("read")),
                Rule::String(String::from("update")),
                Rule::String(String::from("delete")),
                Rule::String(String::from("list")),
            ]))
        );
        assert_eq!(
            Rule::from_str("(if (eq $role admin) (list create read update delete list) (if (eq $role user) (list read list) (list)))")
                .unwrap()
                .eval(&Context::from_str("name:john,role:user").unwrap()),
            Ok(Rule::Tuple(vec![
                Rule::String(String::from("read")),
                Rule::String(String::from("list")),
            ]))
        );
    }
}
