use crate::config::Config;
use crate::permission::{Operation, Permission};
use crate::rule::{Context, RuleError, Rule};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::str::FromStr;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ResourceError {
    #[error("Rule is not starting with a \"/\" '{0}'")]
    FormatError(String),
    #[error("Duplicate resource definition '{0}'")]
    DuplicateResource(String),
    #[error("Ambiguous resource definition '{0}'. {1} is already defined")]
    AmbiguousResource(String, String),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize, Default)]
pub struct ResourceAttributes {
    pub access_rule: Option<Rule>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourcePath(Vec<String>);

impl FromStr for ResourcePath {
    type Err = ResourceError;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        if !path.starts_with('/') {
            return Err(ResourceError::FormatError(path.to_string()));
        }
        let mut clean_path: Vec<char> = path.chars().collect();
        clean_path.dedup();
        let clean_path = clean_path.iter().skip(1).collect::<String>();

        Ok(ResourcePath(
            clean_path
                .split('/')
                .map(std::string::ToString::to_string)
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
    special_child_name: Option<String>,
}

impl ResourceHierarchy {
    #[must_use]
    pub fn new(name: String, attributes: ResourceAttributes) -> Self {
        ResourceHierarchy {
            name,
            attributes,
            children: BTreeMap::new(),
            special_child_name: None,
        }
    }

    pub fn is_allowed(
        &self,
        to: Operation,
        on: &mut ResourcePath,
        with: &Context,
    ) -> Result<bool, RuleError> {
        if let Some(access_rule) = &self.attributes.access_rule {
            let permission: Permission = access_rule.eval(with)?.into();
            if to.allowed_for(permission) {
                return Ok(true);
            }
        }

        let Some(child_name) = on.0.pop() else {
            return Ok(false);
        };

        if let Some(child) = self.children.get("") {
            if let Some(access_rule) = &child.attributes.access_rule {
                let permission: Permission = access_rule.eval(with)?.into();
                if to.allowed_for(permission) {
                    return Ok(true);
                }
            }
        }

        let child_name = if let Some(spechial_child_name) = &self.special_child_name {
            let attribute_value = with.get(spechial_child_name)?;

            if !match (attribute_value, &Rule::from_literal(child_name.as_str())?) {
                (Rule::String(l), Rule::String(r)) => Ok(l == r),
                (Rule::Float(l), Rule::Float(r)) => Ok(l == r),
                (Rule::Integer(l), Rule::Integer(r)) => Ok(l == r),
                (Rule::Bool(l), Rule::Bool(r)) => Ok(l == r),
                (l, r) => Err(RuleError::CannotCompare(l.clone(), r.clone())),
            }? {
                return Ok(false);
            }

            spechial_child_name.clone()
        } else {
            child_name
        };

        if let Some(child) = self.children.get(&child_name) {
            return child.is_allowed(to, on, with);
        }

        Ok(false)
    }

    fn insert(
        &mut self,
        full_path: &str,
        path: &mut ResourcePath,
        attributes: ResourceAttributes,
    ) -> Result<(), ResourceError> {
        if path.0.is_empty() {
            if self.attributes.access_rule.is_some() {
                return Err(ResourceError::DuplicateResource(full_path.to_string()));
            }
            self.attributes = attributes;
            return Ok(());
        }

        let mut child_name = path.0.pop().unwrap();

        if child_name.starts_with(':') {
            if self.special_child_name.is_some() {
                return Err(ResourceError::AmbiguousResource(
                    full_path.to_string(),
                    self.special_child_name.clone().unwrap(),
                ));
            }
            self.special_child_name = Some(child_name[1..].to_string());
            child_name = self.special_child_name.clone().unwrap();
        }

        let child = self.children.entry(child_name.clone()).or_insert_with(|| {
            ResourceHierarchy::new(child_name.clone(), ResourceAttributes::default())
        });

        child.insert(full_path, path, attributes)?;

        Ok(())
    }
}

impl TryFrom<Config> for ResourceHierarchy {
    type Error = ResourceError;

    fn try_from(config: Config) -> Result<Self, ResourceError> {
        let mut root = ResourceHierarchy::new(String::new(), ResourceAttributes::default());

        for (path, attributes) in config.resources {
            root.insert(path.as_str(), &mut ResourcePath::from_str(path.as_str())?, attributes)?;
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
        let left: Result<ResourcePath, ResourceError> = ResourcePath::from_str("/");
        let right: Result<ResourcePath, ResourceError> = Ok(ResourcePath(vec![String::new()]));
        assert_eq!(left, right);

        let left: Result<ResourcePath, ResourceError> = ResourcePath::from_str("/test1/test2");
        let right: Result<ResourcePath, ResourceError> =
            Ok(ResourcePath(vec!["test2".to_string(), "test1".to_string()]));
        assert_eq!(left, right);

        let left: Result<ResourcePath, ResourceError> = ResourcePath::from_str("/test1/test2/");
        let right: Result<ResourcePath, ResourceError> = Ok(ResourcePath(vec![
            String::new(),
            "test2".to_string(),
            "test1".to_string(),
        ]));
        assert_eq!(left, right);
    }

    #[test]
    fn test_resource_hierarchy_insert_err() {
        let mut rh: ResourceHierarchy = toml::from_str::<Config>(
            r#"
            [resources]
            "/:a" = {access_rule = "()"}
        "#,
        ).unwrap().try_into().unwrap();
        assert_eq!(rh.insert(
            "/:b",
            &mut ResourcePath::from_str("/:b").unwrap(),
            ResourceAttributes::default()
        ), Err(ResourceError::AmbiguousResource("/:b".to_string(), "a".to_string())));
    }

    #[test]
    fn test_resource_hierarchy_from_config_ok() {
        let left: Result<ResourceHierarchy, ResourceError> = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "()", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into();
        let right: Result<ResourceHierarchy, ResourceError> = Ok(ResourceHierarchy {
            name: String::new(),
            attributes: ResourceAttributes {
                access_rule: None,
                description: None,
            },
            children: BTreeMap::from([(
                String::new(),
                ResourceHierarchy {
                    name: String::new(),
                    attributes: ResourceAttributes {
                        access_rule: Some(Rule::from_str("()").unwrap()),
                        description: Some("Root".to_string()),
                    },
                    children: BTreeMap::new(),
                    special_child_name: None,
                },
            )]),
            special_child_name: None,
        });
        assert_eq!(left, right);

        let left: Result<ResourceHierarchy, ResourceError> = toml::from_str::<Config>(
            r#"
            [resources]
            "/test" = {access_rule = """(
                list create
            )""", description = "Root"}
            "/test/" = {access_rule = "(list read)", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into();
        let right: Result<ResourceHierarchy, ResourceError> = Ok(ResourceHierarchy {
            name: String::new(),
            attributes: ResourceAttributes {
                access_rule: None,
                description: None,
            },
            children: BTreeMap::from([(
                "test".to_string(),
                ResourceHierarchy {
                    name: "test".to_string(),
                    attributes: ResourceAttributes {
                        access_rule: Some(Rule::from_str("(list create)").unwrap()),
                        description: Some("Root".to_string()),
                    },
                    children: BTreeMap::from([(
                        String::new(),
                        ResourceHierarchy {
                            name: String::new(),
                            attributes: ResourceAttributes {
                                access_rule: Some(Rule::from_str("(list read)").unwrap()),
                                description: Some("Root".to_string()),
                            },
                            children: BTreeMap::new(),
                            special_child_name: None,
                        },
                    )]),
                    special_child_name: None,
                },
            )]),
            special_child_name: None,
        });
        assert_eq!(left, right);
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Sometimes it's ok for tests to be really f-cking long
    fn test_is_allowed_ok() {
        let rh: ResourceHierarchy = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "(list)", description = "Root"}
            "/test1" = {access_rule = "(list create)", description = "Root"}
            "/test1/" = {access_rule = "(list read)", description = "Root"}
            "/test2/test3" = {access_rule = "(list read)", description = "Root"}
            "/all" = {access_rule = "(list all)", description = "Root"}
            "/private/:user_id" = {access_rule = "(list all)", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into()
        .unwrap();

        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut ResourcePath::from_str("/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Create,
                &mut ResourcePath::from_str("/test1").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test1").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Create,
                &mut ResourcePath::from_str("/test1/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test1/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Create,
                &mut ResourcePath::from_str("/test1/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test1/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut ResourcePath::from_str("/test2").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test2").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut ResourcePath::from_str("/test2/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test2/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut ResourcePath::from_str("/test2/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test2/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut ResourcePath::from_str("/test2/test3").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test2/test3").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test2/test3/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut ResourcePath::from_str("/test2/test3/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Delete,
                &mut ResourcePath::from_str("/all").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Delete,
                &mut ResourcePath::from_str("/all/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Delete,
                &mut ResourcePath::from_str("/all/1").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh.is_allowed(
            Operation::Delete,
            &mut ResourcePath::from_str("/private/1").unwrap(),
            &Context::from_str("user_id:1").unwrap()
        ).unwrap());
        assert!(!rh.is_allowed(
            Operation::Delete,
            &mut ResourcePath::from_str("/private/2").unwrap(),
            &Context::from_str("user_id:1").unwrap()
        ).unwrap());
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Sometimes it's ok for tests to be really f-cking long
    fn test_is_allowed_err() {
        let rh: ResourceHierarchy = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "(list)", description = "Root"}
            "/test1" = {access_rule = "(list create)", description = "Root"}
            "/test1/" = {access_rule = "(list read)", description = "Root"}
            "/test2/test3" = {access_rule = "(list read)", description = "Root"}
            "/all" = {access_rule = "(list all)", description = "Root"}
            "/private/:user_id" = {access_rule = "(list all)", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into()
        .unwrap();

        assert_eq!(rh.is_allowed(
            Operation::Delete,
            &mut ResourcePath::from_str("/private/").unwrap(),
            &Context::from_str("user_id:1").unwrap()
        ), Err(RuleError::CannotCompare(Rule::Integer(1), Rule::String("".to_string()))));
        assert_eq!(rh.is_allowed(
            Operation::Delete,
            &mut ResourcePath::from_str("/private/").unwrap(),
            &Context::from_str("").unwrap()
        ), Err(RuleError::KeyNotInContext("user_id".to_string())));
    }
}
