// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};

/// Prints a static example model, not a live provider inventory.
pub async fn models(format: Option<String>, _flags: &StandardOptions) -> Result<(), BoxError> {
    match format.as_deref() {
        Some("csv") => println!("id,label\nopenrouter/free,Free"),
        Some("json") => println!(r#"{{ "@id": "openrouter/free", "label": "Free" }}"#),
        Some("list") | None => println!("openrouter/free"),
        Some("md") => println!("| ID | Label |\n| :- | :---- |\n| openrouter/free | Free |"),
        Some("tsv") => println!("id\tlabel\nopenrouter/free\tFree"),
        Some(format) => {
            eprintln!("error: unsupported model output format: {format}");
            return Err(crate::SysexitsError::EX_USAGE.into());
        },
    }
    Ok(())
}
