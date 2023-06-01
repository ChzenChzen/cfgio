use error_stack::{IntoReport, Result, ResultExt};
use strum::{Display, EnumString};
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum Reason {
    #[error("Failed to parse development environment from value `{0}`")]
    EnvironmentVariableParsing(String),
    #[error("Failed to get access to working directory")]
    WorkingDirectoryAccess,
    #[error("Failed to set config's specifications")]
    Preparation,
    #[error("Failed to compose config schema from sources")]
    ComposeSchema,
    #[error("Failed to deserialize config from schema")]
    Deserialization,
}

#[derive(ThisError, Debug)]
#[error("Failed to build config. Reason: {0}")]
pub struct Error(pub Reason);

#[derive(derive_builder::Builder, Debug, Clone)]
#[builder(build_fn(private, name = "prepare"))]
pub struct Config {
    #[builder(default = r#"String::from("APP_ENV")"#, setter(into))]
    pub environment_variable_name: String,
    #[builder(default = r#"String::from("config")"#, setter(into))]
    pub config_directory: String,
    #[builder(default = r#"String::from("APP")"#, setter(into))]
    pub environment_variables_source_prefix: String,
    #[builder(default = r#"String::from("_")"#, setter(into))]
    pub environment_variables_source_prefix_separator: String,
    #[builder(default = r#"String::from("__")"#, setter(into))]
    pub environment_variables_source_separator: String,
}

impl ConfigBuilder {
    pub fn build<Cfg: serde::de::DeserializeOwned>(&self) -> Result<Cfg, Error> {
        let Config {
            environment_variable_name,
            config_directory,
            environment_variables_source_prefix,
            environment_variables_source_prefix_separator,
            environment_variables_source_separator,
        } = self
            .prepare()
            .into_report()
            .change_context(Error(Reason::Preparation))?;

        let environment = match std::env::var(environment_variable_name) {
            Ok(env) => env
                .parse()
                .into_report()
                .change_context_lazy(|| Error(Reason::EnvironmentVariableParsing(env)))?,
            Err(_) => Environment::default(),
        };

        let working_directory = std::env::current_dir()
            .into_report()
            .change_context(Error(Reason::WorkingDirectoryAccess))?
            .join(config_directory)
            .join(environment.to_string());

        let file_source = config::File::from(working_directory).required(false);
        let env_vars_source =
            config::Environment::with_prefix(&environment_variables_source_prefix)
                .prefix_separator(&environment_variables_source_prefix_separator)
                .separator(&environment_variables_source_separator);

        config::Config::builder()
            .add_source(file_source)
            .add_source(env_vars_source)
            .build()
            .into_report()
            .change_context(Error(Reason::ComposeSchema))?
            .try_deserialize()
            .into_report()
            .change_context(Error(Reason::Deserialization))
    }
}

#[derive(EnumString, Display, Default)]
#[strum(serialize_all = "snake_case")]
pub enum Environment {
    #[default]
    Local,
    Production,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn builder() {
        #[derive(Deserialize)]
        struct FooConfig {
            bar: BarConfig,
        }

        #[derive(Deserialize)]
        struct BarConfig {
            baz: u16,
        }
        std::env::set_var("APP_ENV", "production");
        std::env::set_var("APP_BAR__BAZ", "777");
        let ret: FooConfig = ConfigBuilder::default().build().unwrap();
        assert_eq!(ret.bar.baz, 777);
    }
}
