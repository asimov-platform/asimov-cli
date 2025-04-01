// This is free and unencumbered software released into the public domain.

use temp_dir::TempDir;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct TestFile {
    pub name: &'static str,
    pub content: &'static str,
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
