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
    let toml = r#"
        [resources]
        "/" = {access_rule = "()", description = "Root"}
        "/dataplatform/" = {access_rule = "()", description = "Root"}
    "#;

    let config: Config = toml::from_str(toml).unwrap();
    let resource_hierarchy: ResourceHierarchy = config.try_into().unwrap();
    // println!("{:?}", config);
}

#[cfg(test)]
mod tests {
    use super::*;
}
