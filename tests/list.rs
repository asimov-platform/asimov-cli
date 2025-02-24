use asimov_cli::ExternalCommands;
use temp_dir::TempDir;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct TestFile {
    name: &'static str,
    content: &'static str,
    should_be_listed: bool,
    #[allow(dead_code)]
    win_ext: &'static str,
}

impl TestFile {
    fn full_name(&self) -> String {
        #[cfg(windows)]
        return format!("{}.{}", self.name, self.win_ext);
        #[cfg(unix)]
        return self.name.to_string();
    }
}

static TEST_FILES: &[TestFile] = &[
    TestFile {
        name: "asimov-hello",
        content: "Hello, world!",
        should_be_listed: true,
        win_ext: "bat",
    },
    TestFile {
        name: "asimov-two-levels",
        content: "Should be filtered out!",
        should_be_listed: false,
        win_ext: "bat",
    },
    TestFile {
        name: "abcdefg-test",
        content: "Shouldn't appear!",
        should_be_listed: false,
        win_ext: "bat",
    },
    #[cfg(windows)]
    TestFile {
        name: "asimov-hola",
        content: "Hola mundo!",
        should_be_listed: true,
        win_ext: "cmd",
    },
];

pub fn init() -> Result<TempDir> {
    let dir = TempDir::new()?;

    std::env::set_var("PATH", dir.path());

    #[cfg(unix)]
    for file in TEST_FILES {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;

        let content = format!("#!/bin/sh\n{}", file.content);
        let path = dir.child(file.name);
        let mut file = OpenOptions::new()
            .write(true)
            .mode(0o755)
            .truncate(true)
            .create(true)
            .open(&path)?;
        file.write_all(content.as_bytes())?;
    }

    #[cfg(windows)]
    for file in TEST_FILES {
        let name = file.full_name();
        let content = format!("@echo off\n{}", file.content);
        std::fs::write(dir.child(name), content)?;
    }

    Ok(dir)
}

#[test]
pub fn test_list() -> Result<()> {
    let dir = init()?;
    let cmds = ExternalCommands::collect("asimov-", 1)?;

    for file in TEST_FILES {
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
