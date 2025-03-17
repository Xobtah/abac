mod rule;
mod permission;

use rule::Rule;
use permission::{Permission, Operation};

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
