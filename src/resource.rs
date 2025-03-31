use crate::config::Config;
use crate::permission::{Operation, Permission};
use crate::rule::{self, Context, Rule};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::str::FromStr;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Rule is not starting with a \"/\" '{0}'")]
    FormatError(String),
    #[error("Duplicate resource definition '{0}'")]
    DuplicateResource(String),
    #[error("Ambiguous resource definition '{0}'. {1} is already defined")]
    AmbiguousResource(String, String),
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize, Default)]
pub struct Attributes {
    pub access_rule: Option<Rule>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Path(Vec<String>);

impl FromStr for Path {
    type Err = Error;

    fn from_str(path: &str) -> Result<Self, Self::Err> {
        if !path.starts_with('/') {
            return Err(Error::FormatError(path.to_string()));
        }
        let mut clean_path: Vec<char> = path.chars().collect();
        clean_path.dedup();
        let clean_path = clean_path.iter().skip(1).collect::<String>();

        Ok(Path(
            clean_path
                .split('/')
                .map(std::string::ToString::to_string)
                .rev()
                .collect::<Vec<String>>(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hierarchy {
    name: String,
    attributes: Attributes,
    children: BTreeMap<String, Hierarchy>,
    special_child_name: Option<String>,
}

impl Hierarchy {
    #[must_use]
    pub fn new(name: String, attributes: Attributes) -> Self {
        Hierarchy {
            name,
            attributes,
            children: BTreeMap::new(),
            special_child_name: None,
        }
    }

    pub fn is_allowed(
        &self,
        to: Operation,
        on: &mut Path,
        with: &Context,
    ) -> Result<bool, rule::Error> {
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
                (l, r) => Err(rule::Error::CannotCompare(l.clone(), r.clone())),
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
        path: &mut Path,
        attributes: Attributes,
    ) -> Result<(), Error> {
        if path.0.is_empty() {
            if self.attributes.access_rule.is_some() {
                return Err(Error::DuplicateResource(full_path.to_string()));
            }
            self.attributes = attributes;
            return Ok(());
        }

        let mut child_name = path.0.pop().unwrap();

        if child_name.starts_with(':') {
            if self.special_child_name.is_some() {
                return Err(Error::AmbiguousResource(
                    full_path.to_string(),
                    self.special_child_name.clone().unwrap(),
                ));
            }
            self.special_child_name = Some(child_name[1..].to_string());
            child_name = self.special_child_name.clone().unwrap();
        }

        let child = self
            .children
            .entry(child_name.clone())
            .or_insert_with(|| Hierarchy::new(child_name.clone(), Attributes::default()));

        child.insert(full_path, path, attributes)?;

        Ok(())
    }
}

impl TryFrom<Config> for Hierarchy {
    type Error = Error;

    fn try_from(config: Config) -> Result<Self, Error> {
        let mut root = Hierarchy::new(String::new(), Attributes::default());

        for (path, attributes) in config.resources {
            root.insert(
                path.as_str(),
                &mut Path::from_str(path.as_str())?,
                attributes,
            )?;
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
        let left: Result<Path, Error> = Path::from_str("/");
        let right: Result<Path, Error> = Ok(Path(vec![String::new()]));
        assert_eq!(left, right);

        let left: Result<Path, Error> = Path::from_str("/test1/test2");
        let right: Result<Path, Error> = Ok(Path(vec!["test2".to_string(), "test1".to_string()]));
        assert_eq!(left, right);

        let left: Result<Path, Error> = Path::from_str("/test1/test2/");
        let right: Result<Path, Error> = Ok(Path(vec![
            String::new(),
            "test2".to_string(),
            "test1".to_string(),
        ]));
        assert_eq!(left, right);
    }

    #[test]
    fn test_resource_hierarchy_insert_err() {
        let mut rh: Hierarchy = toml::from_str::<Config>(
            r#"
            [resources]
            "/:a" = {access_rule = "()"}
        "#,
        )
        .unwrap()
        .try_into()
        .unwrap();
        assert_eq!(
            rh.insert(
                "/:b",
                &mut Path::from_str("/:b").unwrap(),
                Attributes::default()
            ),
            Err(Error::AmbiguousResource("/:b".to_string(), "a".to_string()))
        );
    }

    #[test]
    fn test_resource_hierarchy_from_config_ok() {
        let left: Result<Hierarchy, Error> = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "()", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into();
        let right: Result<Hierarchy, Error> = Ok(Hierarchy {
            name: String::new(),
            attributes: Attributes {
                access_rule: None,
                description: None,
            },
            children: BTreeMap::from([(
                String::new(),
                Hierarchy {
                    name: String::new(),
                    attributes: Attributes {
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

        let left: Result<Hierarchy, Error> = toml::from_str::<Config>(
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
        let right: Result<Hierarchy, Error> = Ok(Hierarchy {
            name: String::new(),
            attributes: Attributes {
                access_rule: None,
                description: None,
            },
            children: BTreeMap::from([(
                "test".to_string(),
                Hierarchy {
                    name: "test".to_string(),
                    attributes: Attributes {
                        access_rule: Some(Rule::from_str("(list create)").unwrap()),
                        description: Some("Root".to_string()),
                    },
                    children: BTreeMap::from([(
                        String::new(),
                        Hierarchy {
                            name: String::new(),
                            attributes: Attributes {
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
        let rh: Hierarchy = toml::from_str::<Config>(
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
                &mut Path::from_str("/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Create,
                &mut Path::from_str("/test1").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test1").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Create,
                &mut Path::from_str("/test1/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test1/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Create,
                &mut Path::from_str("/test1/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test1/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut Path::from_str("/test2").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test2").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut Path::from_str("/test2/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test2/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut Path::from_str("/test2/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test2/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Create,
                &mut Path::from_str("/test2/test3").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test2/test3").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test2/test3/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Read,
                &mut Path::from_str("/test2/test3/test").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Delete,
                &mut Path::from_str("/all").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Delete,
                &mut Path::from_str("/all/").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Delete,
                &mut Path::from_str("/all/1").unwrap(),
                &Context::from_str("").unwrap()
            )
            .unwrap());
        assert!(rh
            .is_allowed(
                Operation::Delete,
                &mut Path::from_str("/private/1").unwrap(),
                &Context::from_str("user_id:1").unwrap()
            )
            .unwrap());
        assert!(!rh
            .is_allowed(
                Operation::Delete,
                &mut Path::from_str("/private/2").unwrap(),
                &Context::from_str("user_id:1").unwrap()
            )
            .unwrap());
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Sometimes it's ok for tests to be really f-cking long
    fn test_is_allowed_err() {
        let rh: Hierarchy = toml::from_str::<Config>(
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

        assert_eq!(
            rh.is_allowed(
                Operation::Delete,
                &mut Path::from_str("/private/").unwrap(),
                &Context::from_str("user_id:1").unwrap()
            ),
            Err(rule::Error::CannotCompare(
                Rule::Integer(1),
                Rule::String("".to_string())
            ))
        );
        assert_eq!(
            rh.is_allowed(
                Operation::Delete,
                &mut Path::from_str("/private/").unwrap(),
                &Context::from_str("").unwrap()
            ),
            Err(rule::Error::KeyNotInContext("user_id".to_string()))
        );
    }
}
