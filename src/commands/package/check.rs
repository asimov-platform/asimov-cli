// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use color_print::cprintln;
use core::error::Error;

pub async fn check(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    Ok(())
}
