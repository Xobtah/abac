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

fn main() {
}
