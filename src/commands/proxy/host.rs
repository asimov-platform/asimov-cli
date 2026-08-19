// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions};
use std::net::IpAddr;

pub async fn host(_flags: &StandardOptions) -> Result<(), BoxError> {
    let host: IpAddr = std::env::var("ASIMOV_PROXY_BIND") // TODO: resolve the host
        .ok()
        .and_then(|input| input.parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));
    println!("{}", host);
    Ok(())
}
