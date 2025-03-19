use crate::rule::Rule;
use std::str::FromStr;

pub type Permission = u8;

impl From<Rule> for Permission {
    fn from(rule: Rule) -> Self {
        let items = if let Rule::Tuple(items) = rule {
            items
        } else {
            return 0;
        };

        let mut permission = 0;
        for item in items {
            let operation = if let Rule::String(operation) = item {
                operation
            } else {
                return 0;
            };

            let operation = if let Ok(operation) = Operation::from_str(&operation) {
                operation
            } else if operation == "all" {
                return 0b11111;
            } else {
                return 0;
            };

            permission |= <Operation as Into<Permission>>::into(operation);
        }
        permission
    }
}

#[derive(Debug, Clone)]
pub enum Operation {
    Create,
    Read,
    Update,
    Delete,
    List,
}

impl Into<Permission> for Operation {
    fn into(self) -> Permission {
        match self {
            Operation::Create => 0b00001,
            Operation::Read => 0b00010,
            Operation::Update => 0b00100,
            Operation::Delete => 0b01000,
            Operation::List => 0b10000,
        }
    }
}

impl ToString for Operation {
    fn to_string(&self) -> String {
        match self {
            Operation::Create => "create".to_string(),
            Operation::Read => "read".to_string(),
            Operation::Update => "update".to_string(),
            Operation::Delete => "delete".to_string(),
            Operation::List => "list".to_string(),
        }
    }
}

impl FromStr for Operation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "create" => Ok(Operation::Create),
            "read" => Ok(Operation::Read),
            "update" => Ok(Operation::Update),
            "delete" => Ok(Operation::Delete),
            "list" => Ok(Operation::List),
            _ => Err(()),
        }
    }
}

impl Operation {
    pub fn allowed_for(&self, permission: Permission) -> bool {
        match self {
            Operation::Create => permission & <Operation as Into<Permission>>::into(self.clone()) != 0,
            Operation::Read => permission & <Operation as Into<Permission>>::into(self.clone()) != 0,
            Operation::Update => permission & <Operation as Into<Permission>>::into(self.clone()) != 0,
            Operation::Delete => permission & <Operation as Into<Permission>>::into(self.clone()) != 0,
            Operation::List => permission & <Operation as Into<Permission>>::into(self.clone()) != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::rule::{Rule, Context};
    use std::str::FromStr;
    
    use super::*;

    #[test]
    fn test_permission_from_rule_ok() {
        assert_eq!(
            Permission::from(
                Rule::from_str("()")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            0
        );
        assert_eq!(
            Permission::from(
                Rule::from_str("(list create)")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            <Operation as Into<Permission>>::into(Operation::Create)
        );
        assert_eq!(
            Permission::from(
                Rule::from_str("(list read)")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            <Operation as Into<Permission>>::into(Operation::Read)
        );
        assert_eq!(
            Permission::from(
                Rule::from_str("(list update)")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            <Operation as Into<Permission>>::into(Operation::Update)
        );
        assert_eq!(
            Permission::from(
                Rule::from_str("(list delete)")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            <Operation as Into<Permission>>::into(Operation::Delete)
        );
        assert_eq!(
            Permission::from(
                Rule::from_str("(list list)")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            <Operation as Into<Permission>>::into(Operation::List)
        );
        assert_eq!(
            Permission::from(
                Rule::from_str("(list delete update)")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            <Operation as Into<Permission>>::into(Operation::Delete)
                | <Operation as Into<Permission>>::into(Operation::Update)
        );
        assert_eq!(
            Permission::from(
                Rule::from_str("(list create read update delete)")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            <Operation as Into<Permission>>::into(Operation::Create)
                | <Operation as Into<Permission>>::into(Operation::Read)
                | <Operation as Into<Permission>>::into(Operation::Update)
                | <Operation as Into<Permission>>::into(Operation::Delete)
        );
        assert_eq!(
            Permission::from(
                Rule::from_str("(list all)")
                    .unwrap()
                    .eval(&Context::from_str("").unwrap())
                    .unwrap()
            ),
            <Operation as Into<Permission>>::into(Operation::Create)
                | <Operation as Into<Permission>>::into(Operation::Read)
                | <Operation as Into<Permission>>::into(Operation::Update)
                | <Operation as Into<Permission>>::into(Operation::Delete)
                | <Operation as Into<Permission>>::into(Operation::List)
        );
    }

    #[test]
    fn test_operation_into_permission() {
        let create: Permission = Operation::Create.into();
        let read: Permission = Operation::Read.into();
        let update: Permission = Operation::Update.into();
        let delete: Permission = Operation::Delete.into();
        let list: Permission = Operation::List.into();

        assert_eq!(create, 0b00001);
        assert_eq!(read, 0b00010);
        assert_eq!(update, 0b00100);
        assert_eq!(delete, 0b01000);
        assert_eq!(list, 0b10000);
    }

    #[test]
    fn test_operation_allowed() {
        let permission: Permission = 0b11111;

        assert_eq!(Operation::Create.allowed_for(permission), true);
        assert_eq!(Operation::Read.allowed_for(permission), true);
        assert_eq!(Operation::Update.allowed_for(permission), true);
        assert_eq!(Operation::Delete.allowed_for(permission), true);
        assert_eq!(Operation::List.allowed_for(permission), true);
    }
}
