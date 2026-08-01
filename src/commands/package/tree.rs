// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use core::error::Error;
use treelog::{Tree, renderer::write_tree};

pub async fn tree(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let tree = Tree::from_dir(".")?;

    let mut output = String::new();
    write_tree(&mut output, &tree)?;
    print!("{}", output);

    Ok(())
}
