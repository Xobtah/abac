mod rule;
mod permission;

use std::str::FromStr;
use rule::{Rule, Context};
use permission::{Permission, Operation};

struct Scope {
    name: String,
    description: String,
}

struct Request {
    scope: String,
    operation: String,
    attributes: String,
}

fn main() {
    Context::from_str("").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let attributes = "name:John,age:20,weight:70.5,active:true";
   }
}