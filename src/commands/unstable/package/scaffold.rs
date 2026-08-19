// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*};
use asimov_module_kit::module::{
    NewModuleError, NewModuleOptions,
    lint::{LintOptions, Severity, lint_module},
    new_module,
};
use color_print::{ceprintln, cprintln};
use std::path::PathBuf;

pub async fn new(
    name: &str,
    dir: Option<&str>,
    programs: &[String],
    summary: Option<&str>,
    flags: &StandardOptions,
) -> Result<(), BoxError> {
    let target_dir: PathBuf = match dir {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(format!("asimov-{name}-module")),
    };

    let mut options = NewModuleOptions::new(&target_dir, name)
        .programs(programs.iter().map(|kind| format!("asimov-{name}-{kind}")));
    options.module_summary = Some(
        summary
            .map(String::from)
            .unwrap_or_else(|| format!("ASIMOV {name} module.")),
    );

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
    if !created.program_names.is_empty() {
        cprintln!(
            "<s,dim>hint:</> Programs: <s>{}</>",
            created.program_names.join(", ")
        );
    }

    let mut has_lint_errors = false;
    match lint_module(LintOptions::new(&created.target_dir)) {
        Ok(findings) => {
            for finding in &findings {
                match finding.severity {
                    Severity::Error => {
                        has_lint_errors = true;
                        ceprintln!("<s,r>lint error:</> {}", finding.message);
                    },
                    Severity::Warning if flags.verbose > 0 => {
                        cprintln!("<s,y>lint warning:</> {}", finding.message)
                    },
                    Severity::Warning => {},
                }
            }
        },
        Err(e) => {
            ceprintln!("<s,y>warning:</> failed to lint the generated module: {e}");
        },
    }

    if has_lint_errors {
        return Err(EX_SOFTWARE.into());
    }

    Ok(())
}
