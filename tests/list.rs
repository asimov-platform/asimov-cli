// This is free and unencumbered software released into the public domain.

use clientele::SubcommandsProvider;

mod shared;
use shared::{Result, TEST_FILES, TEST_PREFIX};

#[test]
pub fn test_list() -> Result<()> {
    shared::run_isolated("test_list", check_list)
}

fn check_list(dir: &std::path::Path) -> Result<()> {
    let cmds = SubcommandsProvider::collect(TEST_PREFIX, 1);

    for file in TEST_FILES {
        println!("{}: ", file.name);

        let cd_name = file.name.trim_start_matches(TEST_PREFIX);
        let cmd = cmds.iter().find(|cmd| cmd.name == cd_name);
        let path = dir.join(file.full_name());

        assert_eq!(cmd.is_some(), file.should_be_listed);

        if let Some(cmd) = cmd {
            assert_eq!(cmd.path, path);
        }
    }

    Ok(())
}
