use crate::resource::ResourceAttributes;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Config {
    pub resources: std::collections::HashMap<String, ResourceAttributes>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::Rule;
    use crate::resource::{ResourceAttributes, ResourceHierarchy};
    use std::str::FromStr;
    use toml;
    
    #[test]
    fn test_config_deserialization_ok() {
        let left: Result<Config, toml::de::Error> = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "()", description = "Root"}
        "#,
        );
        let right: Result<Config, toml::de::Error> = Ok(Config {
            resources: std::collections::HashMap::from_iter(vec![(
                "/".to_string(),
                ResourceAttributes {
                    access_rule: Some(Rule::from_str("()").unwrap()),
                    description: Some("Root".to_string()),
                },
            )]),
        });
        assert_eq!(left, right);

        let left: Result<Config, toml::de::Error> = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "()", description = "Root"}
            "/dataplatform/" = {access_rule = "()", description = "Root"}
            "/dataplatform" = {access_rule = "()", description = "Root"}
        "#,
        );
        let right: Result<Config, toml::de::Error> = Ok(Config {
            resources: std::collections::HashMap::from_iter(vec![
                (
                    "/".to_string(),
                    ResourceAttributes {
                        access_rule: Some(Rule::from_str("()").unwrap()),
                        description: Some("Root".to_string()),
                    },
                ),
                (
                    "/dataplatform/".to_string(),
                    ResourceAttributes {
                        access_rule: Some(Rule::from_str("()").unwrap()),
                        description: Some("Root".to_string()),
                    },
                ),
                (
                    "/dataplatform".to_string(),
                    ResourceAttributes {
                        access_rule: Some(Rule::from_str("()").unwrap()),
                        description: Some("Root".to_string()),
                    },
                ),
            ]),
        });
        assert_eq!(left, right);
    }

    #[test]
    fn test_config_deserialization_err() {
        let right = "duplicate key `/` in table `resources`".to_string();
        let left: String = if let Err(error) = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "()", description = "Root"}
            "/" = {access_rule = "()", description = "Root"}
        "#,
        ) {
            error.message().to_string()
        } else {
            panic!("Expected error {:?}", right)
        };
        assert_eq!(left, right);
    }
}
