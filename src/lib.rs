pub mod config;
pub mod permission;
pub mod resource;
pub mod rule;

#[cfg(test)]
mod tests {
    use crate::{config::Config, resource::ResourceHierarchy};

    #[test]
    fn main_test() {
        let rh: ResourceHierarchy = toml::from_str::<Config>(
            r#"
            [resources]
            "/" = {access_rule = "(if true (list all) (list))", description = "Root"}
            "/test" = {access_rule = "(list create)", description = "Root"}
            "/test/" = {access_rule = "(list read)", description = "Root"}
        "#,
        )
        .unwrap()
        .try_into()
        .unwrap();
    }
}
