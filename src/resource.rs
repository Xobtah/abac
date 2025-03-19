use crate::config::Config;
use crate::permission::Operation;
use crate::rule::{Context, Rule};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
pub struct ResourceAttributes {
    pub access_rule: Option<Rule>,
    pub description: Option<String>,
}

impl Default for ResourceAttributes {
    fn default() -> Self {
        ResourceAttributes {
            access_rule: None,
            description: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResourceHierarchy {
    name: String,
    attributes: ResourceAttributes,
    children: BTreeMap<String, ResourceHierarchy>,
}

impl ResourceHierarchy {
    pub fn new(name: String, attributes: ResourceAttributes) -> Self {
        ResourceHierarchy {
            name,
            attributes,
            children: BTreeMap::new(),
        }
    }

    fn is_allowed(&self, to: Operation, on: &str, with: Context) -> bool {
        false
    }

    fn insert(&mut self, path: String, attributes: ResourceAttributes) -> Result<(), ()> {
        let mut parts = path.split('/').map(|s| s.to_string()).rev().collect::<Vec<String>>();
        self._insert(&mut parts, attributes)
    }

    fn _insert(&mut self, path: &mut Vec<String>, attributes: ResourceAttributes) -> Result<(), ()> {
        if path.is_empty() {
            self.attributes = attributes;
            return Ok(());
        }
        
        let child_name = path.pop().unwrap();

        let child = self.children.entry(child_name.clone()).or_insert_with(|| {
            ResourceHierarchy::new(child_name.clone(), ResourceAttributes::default())
        });

        child._insert(path, attributes)?;

        Ok(())
    }
}

impl TryFrom<Config> for ResourceHierarchy {
    type Error = ();

    fn try_from(config: Config) -> Result<Self, ()> {
        let mut root = ResourceHierarchy::new("".to_string(), ResourceAttributes::default());

        for (path, attributes) in config.resources {
            if !path.starts_with('/') {
                return Err(());
            }
            let mut clean_path: Vec<char> = path.chars().collect();
            clean_path.dedup();
            let clean_path = clean_path.iter().skip(1).collect::<String>();
            root.insert(clean_path, attributes)?;
        }
        Ok(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::rule::Rule;
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
                access_rule: None,
                description: None,
            },
            children: BTreeMap::from([(
                "".to_string(),
                ResourceHierarchy {
                    name: "".to_string(),
                    attributes: ResourceAttributes {
                        access_rule: Some(Rule::from_str("()").unwrap()),
                        description: Some("Root".to_string()),
                    },
                    children: BTreeMap::new(),
                },
            )]),
        });
        assert_eq!(left, right);

        let left: Result<ResourceHierarchy, ()> = toml::from_str::<Config>(
            r#"
            [resources]
            "/test" = {access_rule = "(list create)", description = "Root"}
            "/test/" = {access_rule = "(list read)", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into();
        let right: Result<ResourceHierarchy, ()> = Ok(ResourceHierarchy {
            name: "".to_string(),
            attributes: ResourceAttributes {
                access_rule: None,
                description: None,
            },
            children: BTreeMap::from([
                (
                    "test".to_string(),
                    ResourceHierarchy {
                        name: "test".to_string(),
                        attributes: ResourceAttributes {
                            access_rule: Some(Rule::from_str("(list create)").unwrap()),
                            description: Some("Root".to_string()),
                        },
                        children: BTreeMap::from([(
                            "".to_string(),
                            ResourceHierarchy {
                                name: "".to_string(),
                                attributes: ResourceAttributes {
                                    access_rule: Some(Rule::from_str("(list read)").unwrap()),
                                    description: Some("Root".to_string()),
                                },
                                children: BTreeMap::new(),
                            },
                        )]),
                    },
                ),
            ]),
        });
        assert_eq!(left, right);
    }
}
