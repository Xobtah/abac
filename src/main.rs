use abac::permission::{Operation, Permission};
use abac::rule::{Context, Rule};
use abac::resource::{ResourceAttributes, ResourceHierarchy};
use abac::config::Config;
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


}

#[cfg(test)]
mod tests {
    use super::*;
}
