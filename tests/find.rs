// This is free and unencumbered software released into the public domain.

use clientele::SubcommandsProvider;

mod shared;
use shared::{Result, TEST_FILES, TEST_PREFIX};

#[test]
pub fn test_find() -> Result<()> {
    let dir = shared::init()?;

    for file in TEST_FILES {
        println!("{}: ", file.name);

        let cd_name = file.name.trim_start_matches(TEST_PREFIX);
        let cmd = SubcommandsProvider::find(TEST_PREFIX, cd_name);
        let path = dir.child(file.full_name());

        // Direct lookup supports nested names omitted from the root listing.
        assert_eq!(
            cmd.is_some(),
            file.name.starts_with(TEST_PREFIX),
            "unexpected lookup result for {}",
            file.name
        );

        if let Some(cmd) = cmd {
            assert_eq!(cmd.path, path);
        }
    }

    assert!(SubcommandsProvider::find(TEST_PREFIX, "missing-command").is_none());

    Ok(())
}
