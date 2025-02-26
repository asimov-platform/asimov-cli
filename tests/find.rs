use asimov_cli::ExternalCommands;

mod shared;
use shared::{Result, TEST_FILES};

#[test]
pub fn test_find() -> Result<()> {
    let dir = shared::init()?;

    for file in TEST_FILES {
        println!("{}: ", file.name);

        let cd_name = file.name.trim_start_matches("asimov-");
        let cmd = ExternalCommands::find("asimov-", cd_name);
        let path = dir.child(file.full_name());

        // assert_eq!(cmd.is_some(), file.should_be_listed);

        if let Some(cmd) = cmd {
            assert_eq!(cmd.path, path);
        }
    }

    Ok(())
}
