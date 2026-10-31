// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, shared::outln};

/// Prints a static example model, not a live provider inventory.
pub async fn models(format: Option<String>, _flags: &StandardOptions) -> Result<(), BoxError> {
    match format.as_deref() {
        Some("csv") => outln!("id,label\nopenrouter/free,Free")?,
        Some("json") => outln!(r#"{{ "@id": "openrouter/free", "label": "Free" }}"#)?,
        Some("list") | None => outln!("openrouter/free")?,
        Some("md") => outln!("| ID | Label |\n| :- | :---- |\n| openrouter/free | Free |")?,
        Some("tsv") => outln!("id\tlabel\nopenrouter/free\tFree")?,
        Some(format) => {
            eprintln!("error: unsupported model output format: {format}");
            return Err(crate::SysexitsError::EX_USAGE.into());
        },
    }
    Ok(())
}
