use abac::{
    config::Config,
    permission::Operation,
    resource::{Hierarchy, Path},
    rule::Context,
};
use clap::Parser;
use std::{fs, io, path::PathBuf, str::FromStr};

/// ABAC CLI
#[derive(Parser)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = None)]
    config: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("TOML error: {0}")]
    TomlDe(#[from] toml::de::Error),
    #[error("No configuration file")]
    NoConf,
    #[error("Resource error: {0}")]
    Resource(#[from] abac::resource::Error),
    #[error("Rule error: {0}")]
    Rule(#[from] abac::rule::Error),
}

fn main() -> Result<(), Error> {
    let Some(config) = Args::parse().config else {
        return Err(Error::NoConf);
    };

    let rh =
        <Config as TryInto<Hierarchy>>::try_into(toml::from_str(&fs::read_to_string(&config)?)?)?;

    println!(
        "{}",
        rh.is_allowed(
            Operation::Create,
            &mut Path::from_str("/private/2")?,
            &Context::from_str("user_id:1,role:admin")?,
        )
        .unwrap()
    );

    Ok(())
}
