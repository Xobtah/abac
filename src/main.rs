use abac::{config::Config, resource::ResourceHierarchy};

fn main() {
    <Config as TryInto<ResourceHierarchy>>::try_into(
        toml::from_str::<Config>(
            r#"
        [resources]
        "/" = {access_rule = "(if true (list all) (list))", description = "Root"}
        "/test" = {access_rule = "(list create)", description = "Root"}
        "/test/" = {access_rule = "(list read)", description = "Root"}
    "#,
        )
        .unwrap(),
    )
    .unwrap();
}
