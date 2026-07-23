// This is free and unencumbered software released into the public domain.

use crate::{StandardOptions, SysexitsError::*};
use asimov_module_kit::module::{NewModuleError, NewModuleOptions, new_module};
use color_print::{ceprintln, cprintln};
use core::error::Error;
use std::path::{Path, PathBuf};

pub async fn new(
    name: &str,
    dir: Option<&str>,
    program: Option<&str>,
    template: Option<&str>,
    branch: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    let target_dir: PathBuf = match dir {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(format!("asimov-{name}-module")),
    };

    let mut options = NewModuleOptions::new(&target_dir, name);
    match program {
        Some(program) => options.program_name = Some(program.to_string()),
        None => options = options.without_program(),
    }
    if let Some(template) = template {
        options = if Path::new(template).is_dir() {
            options.template_path(template)
        } else {
            options.template_git(template)
        };
    }
    if let Some(branch) = branch {
        options.branch = Some(branch.to_string());
    }

    if flags.verbose > 1 {
        cprintln!(
            "<s,c>»</> Generating module <s>{name}</> in <s>{}</>...",
            target_dir.display()
        );
    }

    let created = new_module(options).map_err(|e| {
        ceprintln!("<s,r>error:</> failed to generate module `{name}`: {e}");
        match e {
            NewModuleError::TargetExists(_) => EX_CANTCREAT,
            NewModuleError::EmptyName
            | NewModuleError::InvalidName(_)
            | NewModuleError::InvalidProgramName(_) => EX_USAGE,
            NewModuleError::CargoGenerate(_) => EX_UNAVAILABLE,
            _ => EX_SOFTWARE,
        }
    })?;

    cprintln!(
        "<s,g>✓</> Created module <s>{}</> at <s>{}</>.",
        created.crate_name,
        created.target_dir.display()
    );
    if let Some(program_name) = created.program_names.first() {
        cprintln!("<s,dim>hint:</> Initial program: <s>{program_name}</>");
    }

    Ok(())
}
