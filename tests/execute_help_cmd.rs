// This is free and unencumbered software released into the public domain.

use asimov_cli::commands::HelpCmd;
use clientele::SysexitsError::*;

mod shared;
use shared::{Result, TEST_FILES, TEST_PREFIX};

#[test]
pub fn test_execute_help_cmd() -> Result<()> {
    let _dir = shared::init()?;

    for file in TEST_FILES.iter() {
        println!("{}: ", file.name);

        let external_cmd = HelpCmd { is_debug: false };

        let cd_name = file.name.trim_start_matches(TEST_PREFIX);
        let result = external_cmd.execute(cd_name, &[]);
        if !file.name.starts_with(TEST_PREFIX) {
            assert!(matches!(result, Err(EX_UNAVAILABLE)), "{}", file.name);
            continue;
        }

        let result = result.unwrap_or_else(|error| panic!("{}: {error}", file.name));
        assert!(
            result.success,
            "{}: {}",
            file.name,
            String::from_utf8_lossy(&result.output)
        );
        assert_eq!(result.code, EX_OK, "{}", file.name);
        assert_eq!(std::str::from_utf8(&result.output)?.trim(), file.help);
    }

    Ok(())
}
