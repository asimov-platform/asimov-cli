// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use core::error::Error;

pub async fn models(
    format: &Option<String>,
    _flags: &StandardOptions,
) -> Result<(), Box<dyn Error>> {
    match format.as_deref() {
        Some("csv") => println!("id,label\nopenrouter/free,Free"),
        Some("json") => println!("{}", r#"{ "@id": "openrouter/free", "label": "Free" }"#),
        Some("list") | None => println!("openrouter/free"),
        Some("md") => println!("| ID | Label |\n| :- | :---- |\n| openrouter/free | Free |"),
        Some("tsv") => println!("id\tlabel\nopenrouter/free\tFree"),
        Some(_) => {}, // TODO
    }
    Ok(())
}
