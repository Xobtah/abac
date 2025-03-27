use abac::{
    config::Config,
    permission::Operation,
    resource::{ResourceHierarchy, ResourcePath},
    rule::Context,
};
use std::str::FromStr;

fn main() {
    let rh = <Config as TryInto<ResourceHierarchy>>::try_into(
        toml::from_str::<Config>(
            r#"
                [resources]
                "/" = {access_rule = "(if (eq $role admin) (list all) (list))", description = "Root"}
                "/test" = {access_rule = "(list read)"}
                "/private/:user_id" = {access_rule = "(list create)", description = "User space"}
            "#,
        )
        .unwrap(),
    )
    .unwrap();

    println!(
        "{}",
        rh.is_allowed(
            Operation::Create,
            &mut ResourcePath::from_str("/private/2").unwrap(),
            &Context::from_str("user_id:1,role:admin").unwrap()
        )
        .unwrap()
    );
}
