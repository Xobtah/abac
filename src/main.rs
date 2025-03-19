mod config;
mod resource;
mod permission;
mod rule;

use permission::{Operation, Permission};
use rule::{Context, Rule};
use resource::{ResourceAttributes, ResourceHierarchy};
use config::Config;
use serde::Deserialize;
use std::convert::TryFrom;
use std::str::FromStr;
use toml;


fn main() {
    let rh: ResourceHierarchy = toml::from_str::<Config>(
        r#"
        [resources]
        "/" = {access_rule = "(if true (list all) (list))", description = "Root"}
        "/test" = {access_rule = "(list create)", description = "Root"}
        "/test/" = {access_rule = "(list read)", description = "Root"}
    "#,
    )
    .unwrap()
    .try_into().unwrap();

    println!("{}", serde_json::to_string_pretty(&rh).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
}
