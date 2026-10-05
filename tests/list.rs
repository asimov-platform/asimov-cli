// This is free and unencumbered software released into the public domain.

use clientele::SubcommandsProvider;

mod shared;
use shared::{Result, TEST_FILES, TEST_PREFIX};

#[test]
pub fn test_list() -> Result<()> {
    shared::run_isolated("test_list", check_list)
}

fn check_list(dir: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join("asimov-disabled");
        std::fs::write(&path, "#!/bin/sh\nexit 0\n")?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644))?;
    }
    let cmds = SubcommandsProvider::collect(TEST_PREFIX, 1);
    let mut actual: Vec<_> = cmds.iter().map(|cmd| cmd.name.as_str()).collect();
    actual.sort_unstable();
    let mut expected: Vec<_> = TEST_FILES
        .iter()
        .filter(|file| file.should_be_listed)
        .map(|file| file.name.trim_start_matches(TEST_PREFIX))
        .collect();
    expected.sort_unstable();
    assert_eq!(
        actual, expected,
        "root commands must be executable and unique"
    );

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
