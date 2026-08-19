// This is free and unencumbered software released into the public domain.

use super::{MASK, Source, open};
use crate::BoxError;
use asimov_module::ModuleName;
use clientele::StandardOptions;
use color_print::{cformat, cprintln};

/// Shows every declared configuration variable, where its effective value
/// comes from, and the value itself unless it is secret.
pub async fn show(
    module_name: &ModuleName,
    output: &str,
    _flags: &StandardOptions,
) -> Result<(), BoxError> {
    let module = open(module_name).await?;
    let variables = module.variables();

    let mut rows = Vec::with_capacity(variables.len());
    for var in variables {
        let source = module.source(var).await;
        let value = match source {
            Source::Unset => None,
            _ => module
                .manifest
                .variable(&var.name, Some(module.profile))
                .ok(),
        };
        rows.push((var, source, value));
    }

    match output {
        "json" => {
            let vars: Vec<serde_json::Value> = rows
                .iter()
                .map(|(var, source, value)| {
                    serde_json::json!({
                        "name": var.name,
                        "description": var.description,
                        "secret": var.secret,
                        "required": var.is_required(),
                        "environment": var.environment,
                        "source": source.as_str(),
                        "value": value.as_deref().filter(|_| !var.secret),
                    })
                })
                .collect();

            println!("{}", serde_json::to_string_pretty(&vars)?);
        },
        _ => {
            if rows.is_empty() {
                cprintln!(
                    "<s,dim>note:</> module <s>{}</> has no configuration variables",
                    module.name
                );
                return Ok(());
            }

            // Pad on the plain text: colored strings carry escape sequences
            // that formatting widths would count as visible characters.
            let shown: Vec<(String, String)> = rows
                .iter()
                .map(|(var, _, value)| match (value, var.secret) {
                    (Some(_), true) => (MASK.into(), MASK.into()),
                    (Some(value), false) => (value.trim().into(), value.trim().into()),
                    (None, _) if var.is_required() => {
                        ("(required)".into(), cformat!("<s,r>(required)</>"))
                    },
                    (None, _) => ("(unset)".into(), cformat!("<dim>(unset)</>")),
                })
                .collect();

            let name_width = rows
                .iter()
                .map(|(var, ..)| var.name.chars().count())
                .max()
                .unwrap_or(0);
            let value_width = shown
                .iter()
                .map(|(plain, _)| plain.chars().count())
                .max()
                .unwrap_or(0);

            for ((var, source, _), (plain, colored)) in rows.iter().zip(&shown) {
                let padding = " ".repeat(value_width - plain.chars().count());
                cprintln!(
                    "<s>{:<name_width$}</>  {}{}  <dim>{}</>",
                    var.name,
                    colored,
                    padding,
                    source.as_str(),
                );
            }
        },
    }

    Ok(())
}
