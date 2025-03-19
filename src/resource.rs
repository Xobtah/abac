use crate::permission::Operation;
use crate::rule::{Context, Rule};
use crate::config::Config;
use serde::Deserialize;
use std::convert::TryFrom;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ResourceAttributes {
    pub access_rule: Option<Rule>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceHierarchy {
    pub name: String,
    pub attributes: ResourceAttributes,
    pub children: Vec<ResourceHierarchy>,
}

impl ResourceHierarchy {
    fn is_allowed(&self, to: Operation, on: &str, with: Context) -> bool {
        false
    }

    fn insert(&mut self, path: String, scope: ResourceAttributes) -> Result<(), ()> {
        let mut parts = path.split('/');
        let name = parts.next().ok_or(())?;
        Ok(())
    }
}

impl TryFrom<Config> for ResourceHierarchy {
    type Error = ();

    fn try_from(config: Config) -> Result<Self, ()> {
        let mut root = ResourceHierarchy {
            name: "".to_string(),
            attributes: ResourceAttributes {
                access_rule: None,
                description: None,
            },
            children: vec![],
        };

        for (path, attributes) in config.resources {
            root.insert(path, attributes)?;
        }
        Ok(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::Rule;
    use crate::config::Config;
    use std::str::FromStr;
    use toml;
    
    #[test]
    fn test_resource_hierarchy_from_config_ok() {
        let left: Result<ResourceHierarchy, ()> = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "()", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into();
        let right: Result<ResourceHierarchy, ()> = Ok(ResourceHierarchy {
            name: "".to_string(),
            attributes: ResourceAttributes {
                access_rule: Some(Rule::from_str("()").unwrap()),
                description: Some("Root".to_string()),
            },
            children: vec![],
        });
        assert_eq!(left, right);
    }
}
