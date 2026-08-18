// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use core::error::Error;
use std::net::IpAddr;

pub async fn url(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let host: IpAddr = std::env::var("ASIMOV_PROXY_HOST")
        .ok()
        .and_then(|input| input.parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let port = std::env::var("ASIMOV_PROXY_PORT")
        .ok()
        .and_then(|input| input.parse::<u16>().ok())
        .unwrap_or(1920);
    println!("http://{}:{}/v1", host, port);
    Ok(())
}
