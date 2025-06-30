// This is free and unencumbered software released into the public domain.

use indoc::formatdoc;
use temp_dir::TempDir;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct TestFile {
    pub name: &'static str,
    pub content: &'static str,
    pub help: &'static str,
    #[allow(dead_code)]
    pub should_be_listed: bool,
    #[allow(dead_code)]
    pub win_ext: &'static str,
}

impl TestFile {
    pub fn full_name(&self) -> String {
        #[cfg(windows)]
        return format!("{}.{}", self.name, self.win_ext);
        #[cfg(unix)]
        return self.name.to_string();
    }
}

pub static TEST_PREFIX: &str = "asimov-";

pub static TEST_FILES: &[TestFile] = &[
    TestFile {
        name: "asimov-hello",
        content: "Hello, world!",
        help: "Prints 'Hello, world!'",
        should_be_listed: true,
        win_ext: "bat",
    },
    TestFile {
        name: "asimov-two-levels",
        content: "Should be filtered out!",
        help: "This file is not listed",
        should_be_listed: false,
        win_ext: "bat",
    },
    TestFile {
        name: "abcdefg-test",
        content: "Shouldn't appear!",
        help: "This file is not listed too",
        should_be_listed: false,
        win_ext: "bat",
    },
    #[cfg(windows)]
    TestFile {
        name: "asimov-hola",
        content: "Hola mundo!",
        help: "This is windows-only subcommand",
        should_be_listed: true,
        win_ext: "cmd",
    },
];

pub fn init() -> Result<TempDir> {
    let dir = TempDir::new()?;

    std::env::set_var("PATH", dir.path());

    #[cfg(unix)]
    for file in TEST_FILES {
        use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt};

        #[rustfmt::skip]
        let content = formatdoc!(r#"
            #!/bin/sh
            if [ "$1" = "--help" ]; then
                echo "{help}"
            else
                echo "{content}"
            fi"#,
            help = file.help,
            content = file.content,
        );

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

        #[rustfmt::skip]
        let content = formatdoc!(r#"
            @echo off
            if "%~1" == "--help" (
                echo {help}
            ) else (
                echo {content}
            )"#,
            help = file.help,
            content = file.content,
        );

        std::fs::write(dir.child(name), content)?;
    }

    Ok(dir)
}
