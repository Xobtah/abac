use std::{collections::VecDeque, str::FromStr};

mod rule;

type Permission = u8;

enum Operation {
    Create,
    Read,
    Update,
    Delete,
    List,
}

impl Operation {
    fn allowed (&self, permission: Permission) -> bool {
        match self {
            Operation::Create => permission & 0b00001 != 0,
            Operation::Read => permission & 0b00010 != 0,
            Operation::Update => permission & 0b00100 != 0,
            Operation::Delete => permission & 0b01000 != 0,
            Operation::List => permission & 0b10000 != 0,
        }
    }
}

struct Scope {
    name: String,
    description: String,
}

struct Attribute {
    name: String,
    value: String,
}

struct Request {
    operation: Operation,
    scope: Scope,
    attributes: Vec<Attribute>,
}

// (if (eq name john) (set create) (set)))
// (if (eq role admin) (set create read update delete list) (if (eq role user) (set read list) (set)))


fn main() {
    let context1: rule::Context = vec![
        (String::from("name"), rule::Rule::String(String::from("john"))),
        (String::from("role"), rule::Rule::String(String::from("admin"))),
    ];
    let context2: rule::Context = vec![
        (String::from("name"), rule::Rule::String(String::from("john"))),
        (String::from("role"), rule::Rule::String(String::from("user"))),
    ];
    println!("{:?}", rule::Rule::from_str("(if (eq $name john) (set create) (set))").unwrap().eval(&context1));
    println!("{:?}", rule::Rule::from_str("(if (eq $role admin) (set create read update delete list) (if (eq role user) (set read list) (set)))").unwrap().eval(&context1));
    println!("{:?}", rule::Rule::from_str("(if (eq $role admin) (set create read update delete list) (if (eq $role user) (set read list) (set)))").unwrap().eval(&context2));
}
