// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use core::error::Error;

pub async fn port(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let port = std::env::var("ASIMOV_PROXY_PORT")
        .ok()
        .and_then(|input| input.parse::<u16>().ok())
        .unwrap_or(1920);
    println!("{}", port);
    Ok(())
}
