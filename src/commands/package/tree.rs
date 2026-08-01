// This is free and unencumbered software released into the public domain.

use crate::StandardOptions;
use core::error::Error;
use treelog::{Tree, renderer::write_tree};

pub async fn tree(_flags: &StandardOptions) -> Result<(), Box<dyn Error>> {
    let tree = Tree::from_dir(".")?;
    let tree = prune(&tree, &|input| match input {
        Tree::Leaf(lines) => lines.iter().any(|line| line.ends_with("~")),
        Tree::Node(label, _) => match label.as_str() {
            ".git" => true,
            "target" => true,
            _ => false,
        },
    })
    .unwrap_or(tree);

    let mut output = String::new();
    write_tree(&mut output, &tree)?;
    print!("{}", output);

    Ok(())
}

fn prune<F>(input: &Tree, predicate: &F) -> Option<Tree>
where
    F: Fn(&Tree) -> bool,
{
    match input {
        Tree::Node(label, children) => {
            if !predicate(input) {
                Some(Tree::Node(
                    label.clone(),
                    children
                        .into_iter()
                        .filter_map(|child| prune(child, predicate))
                        .collect(),
                ))
            } else {
                None
            }
        },
        Tree::Leaf(lines) => {
            if !predicate(input) {
                Some(Tree::Leaf(lines.clone()))
            } else {
                None
            }
        },
    }
}
