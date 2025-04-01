// This is free and unencumbered software released into the public domain.

use asimov_cli::commands::HelpCmd;

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
        // assert_eq!(result.is_ok(), file.should_be_listed);

        if let Ok(result) = result {
            assert_eq!(result.code, 0);

            assert_eq!(
                std::str::from_utf8(&result.output).unwrap().trim(),
                file.help
            );
        }
    }

    Ok(())
}
