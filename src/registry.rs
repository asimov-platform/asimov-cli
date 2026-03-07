// This is free and unencumbered software released into the public domain.

pub mod crates;
pub mod http;
pub mod pypi;
pub mod rubygems;

use crate::{
    SysexitsError::{self},
    registry,
};
use asimov_env::{
    env::Env,
    envs::{PythonEnv, RubyEnv},
};
use derive_more::Display;

#[derive(Clone, Debug)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: String,
    pub r#type: ModuleType,
    pub url: String,
}

impl ModuleMetadata {
    pub fn is_installed(&self) -> std::io::Result<bool> {
        match self.r#type {
            ModuleType::Rust => {
                let command_name = format!("{}-module", self.name); // FIXME
                Ok(clientele::SubcommandsProvider::find("asimov-", &command_name).is_some())
            },
            ModuleType::Ruby => RubyEnv::default().is_module_installed(&self.name),
            ModuleType::Python => PythonEnv::default().is_module_installed(&self.name),
        }
    }
}

#[derive(Clone, Display, Debug)]
pub enum ModuleType {
    #[display("rust")]
    Rust,
    #[display("ruby")]
    Ruby,
    #[display("python")]
    Python,
}

impl ModuleType {
    pub fn origin(&self) -> &'static str {
        use ModuleType::*;
        match self {
            Rust => "Cargo",
            Ruby => "RubyGems",
            Python => "PyPI",
        }
    }
}

pub fn is_enabled(_module_name: &str) -> bool {
    true // TODO
}

pub async fn fetch_module(module_name: &str) -> Option<ModuleMetadata> {
    let modules = registry::fetch_modules().await.ok()?;
    let module = modules.into_iter().find(|m| m.name == module_name);
    module
}

pub async fn fetch_modules() -> Result<Vec<ModuleMetadata>, SysexitsError> {
    // Disable registry modules temporarily.
    return Ok(Vec::new());

    // // Spawn tasks to fetch module package metadata:
    // let rust_task = task::spawn(async {
    //     let result = registry::crates::fetch_current_modules()
    //         .await
    //         .map_err(|e| {
    //             tracing::error!("failed to fetch Rust module metadata: {e}");
    //             EX_UNAVAILABLE
    //         })?;
    //     registry::crates::extract_module_names(result).map_err(|e| {
    //         tracing::error!("failed to parse Rust module metadata: {e}");
    //         EX_DATAERR
    //     })
    // });
    // let ruby_task = task::spawn(async {
    //     let result = registry::rubygems::fetch_current_modules()
    //         .await
    //         .map_err(|e| {
    //             tracing::error!("failed to fetch Ruby module metadata: {e}");
    //             EX_UNAVAILABLE
    //         })?;
    //     registry::rubygems::extract_module_names(result).map_err(|e| {
    //         tracing::error!("failed to parse Ruby module metadata: {e}");
    //         EX_DATAERR
    //     })
    // });
    // let python_task = task::spawn(async {
    //     let result = registry::pypi::fetch_current_modules().await.map_err(|e| {
    //         tracing::error!("failed to fetch Python module metadata: {e}");
    //         EX_UNAVAILABLE
    //     })?;
    //     registry::pypi::extract_module_names(result).map_err(|e| {
    //         tracing::error!("failed to parse Python module metadata: {e}");
    //         EX_DATAERR
    //     })
    // });

    // // Await all tasks; note the double ?? to handle both `JoinError` and the
    // // `Result` from the task:
    // let rust_modules = rust_task.await.map_err(|e| {
    //     tracing::error!("failed to join Rust module task: {e}");
    //     EX_SOFTWARE
    // })??;
    // let ruby_modules = ruby_task.await.map_err(|e| {
    //     tracing::error!("failed to join Ruby module task: {e}");
    //     EX_SOFTWARE
    // })??;
    // let python_modules = python_task.await.map_err(|e| {
    //     tracing::error!("failed to join Python module task: {e}");
    //     EX_SOFTWARE
    // })??;

    // let mut all_modules: Vec<ModuleMetadata> = rust_modules
    //     .iter()
    //     .chain(ruby_modules.iter())
    //     .chain(python_modules.iter())
    //     .cloned()
    //     .collect();

    // all_modules.sort_by_key(|m| m.name.clone());

    // Ok(all_modules)
}
