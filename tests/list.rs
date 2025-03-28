// This is free and unencumbered software released into the public domain.

use asimov_cli::SubcommandsProvider;

mod shared;
use shared::{Result, TEST_FILES};

#[test]
pub fn test_list() -> Result<()> {
    let dir = shared::init()?;
    let cmds = SubcommandsProvider::collect("asimov-", 1);

    for file in TEST_FILES {
        println!("{}: ", file.name);

        let cd_name = file.name.trim_start_matches("asimov-");
        let cmd = cmds.iter().find(|cmd| cmd.name == cd_name);
        let path = dir.child(file.full_name());

        assert_eq!(cmd.is_some(), file.should_be_listed);

        if let Some(cmd) = cmd {
            assert_eq!(cmd.path, path);
        }
    }

    Ok(())
}
