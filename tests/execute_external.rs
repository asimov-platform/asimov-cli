// This is free and unencumbered software released into the public domain.

use asimov_cli::commands::ExternalSubcommand;
use clientele::SysexitsError::*;

mod shared;
use shared::{Result, TEST_FILES, TEST_PREFIX};

#[test]
pub fn test_execute_external() -> Result<()> {
    shared::run_isolated("test_execute_external", check_execute_external)
}

fn check_execute_external(_dir: &std::path::Path) -> Result<()> {
    for file in TEST_FILES.iter() {
        println!("{}: ", file.name);

        let external_cmd = ExternalSubcommand {
            is_debug: false,
            pipe_output: true,
        };

        let cd_name = file.name.trim_start_matches(TEST_PREFIX);
        let result = external_cmd.execute(cd_name, &[]);
        if !file.name.starts_with(TEST_PREFIX) {
            assert!(matches!(result, Err(EX_UNAVAILABLE)), "{}", file.name);
            continue;
        }

        let result = result.unwrap_or_else(|error| panic!("{}: {error}", file.name));
        let stdout = result.stdout.expect("captured stdout");
        let stderr = result.stderr.expect("captured stderr");
        assert_eq!(
            result.code,
            EX_OK,
            "{}: {}",
            file.name,
            String::from_utf8_lossy(&stderr)
        );
        assert_eq!(std::str::from_utf8(&stdout)?.trim(), file.content);
        assert!(stderr.is_empty(), "{}: {stderr:?}", file.name);
    }

    Ok(())
}
