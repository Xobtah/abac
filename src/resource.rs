use crate::config::Config;
use crate::permission::{Operation, Permission};
use crate::rule::{Context, Rule, Error};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::collections::BTreeMap;
use std::str::FromStr;

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

#[derive(Debug, Clone, PartialEq)]
struct ResourcePath(Vec<String>);

impl FromStr for ResourcePath {
    type Err = ();

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        if !path.starts_with('/') {
            return Err(());
        }
        let mut clean_path: Vec<char> = path.chars().collect();
        clean_path.dedup();
        let clean_path = clean_path.iter().skip(1).collect::<String>();

        Ok(ResourcePath(
            clean_path
                .split('/')
                .map(|s| s.to_string())
                .rev()
                .collect::<Vec<String>>(),
        ))
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

    fn is_allowed(&self, to: Operation, on: &mut ResourcePath, with: &Context) -> Result<bool, Error> {
        if let Some(access_rule) = &self.attributes.access_rule {
            let permission: Permission = access_rule.eval(&with)?.into();
            if to.allowed_for(permission) {
                return Ok(true);
            }
        }

        let child_name  = if let Some(child_name) = on.0.pop() {
            child_name
        } else {
            return Ok(false);
        };

        if let Some(child) = self.children.get("") {
            if let Some(access_rule) = &child.attributes.access_rule {
                let permission: Permission = access_rule.eval(&with)?.into();
                if to.allowed_for(permission) {
                    return Ok(true);
                }
            }
        }

        if let Some(child) = self.children.get(&child_name) {
            return child.is_allowed(to, on, with);
        }

        Ok(false)
    }

    fn insert(&mut self, path: &mut ResourcePath, attributes: ResourceAttributes) -> Result<(), ()> {
        if path.0.is_empty() {
            if self.attributes.access_rule.is_some() {
                return Err(());
            }
            self.attributes = attributes;
            return Ok(());
        }
        
        let child_name = path.0.pop().unwrap();

        let child = self.children.entry(child_name.clone()).or_insert_with(|| {
            ResourceHierarchy::new(child_name.clone(), ResourceAttributes::default())
        });

        child.insert(path, attributes)?;

        Ok(())
    }

}

impl TryFrom<Config> for ResourceHierarchy {
    type Error = ();

    fn try_from(config: Config) -> Result<Self, ()> {
        let mut root = ResourceHierarchy::new("".to_string(), ResourceAttributes::default());

        for (path, attributes) in config.resources {
            root.insert(&mut ResourcePath::from_str(path.as_str())?, attributes)?;
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
    fn test_resource_path_from_str_ok() {
        let left: Result<ResourcePath, ()> = ResourcePath::from_str("/");
        let right: Result<ResourcePath, ()> = Ok(ResourcePath(vec!["".to_string()]));
        assert_eq!(left, right);

        let left: Result<ResourcePath, ()> = ResourcePath::from_str("/test1/test2");
        let right: Result<ResourcePath, ()> = Ok(ResourcePath(vec!["test2".to_string(), "test1".to_string()]));
        assert_eq!(left, right);

        let left: Result<ResourcePath, ()> = ResourcePath::from_str("/test1/test2/");
        let right: Result<ResourcePath, ()> = Ok(ResourcePath(vec!["".to_string(), "test2".to_string(), "test1".to_string()]));
        assert_eq!(left, right);
    }

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

    #[test]
    fn test_is_allowed_ok() {
        let rh: ResourceHierarchy = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "(list)", description = "Root"}
            "/test1" = {access_rule = "(list create)", description = "Root"}
            "/test1/" = {access_rule = "(list read)", description = "Root"}
            "/test2/test3" = {access_rule = "(list read)", description = "Root"}
            "/all" = {access_rule = "(list all)", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(
            rh.is_allowed(Operation::Create, &mut ResourcePath::from_str("/").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Create, &mut ResourcePath::from_str("/test1").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test1").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Create, &mut ResourcePath::from_str("/test1/").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test1/").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Create, &mut ResourcePath::from_str("/test1/test").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test1/test").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Create, &mut ResourcePath::from_str("/test2").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test2").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Create, &mut ResourcePath::from_str("/test2/").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test2/").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Create, &mut ResourcePath::from_str("/test2/test").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test2/test").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Create, &mut ResourcePath::from_str("/test2/test3").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            false
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test2/test3").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test2/test3/").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Read, &mut ResourcePath::from_str("/test2/test3/test").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Delete, &mut ResourcePath::from_str("/all").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Delete, &mut ResourcePath::from_str("/all/").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
        assert_eq!(
            rh.is_allowed(Operation::Delete, &mut ResourcePath::from_str("/all/1").unwrap(), &Context::from_str("").unwrap())
                .unwrap(),
            true
        );
    }
}
