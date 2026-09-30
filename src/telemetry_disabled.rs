// This is free and unencumbered software released into the public domain.

use asimov_id::PublicKey;
use clientele::crates::clap::{ArgMatches, CommandFactory};

pub fn initialize(_: &PublicKey, _: bool) {}

#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Fetch,
    Read,
    List,
}

pub struct CommandMetadata;

impl CommandMetadata {
    pub fn from_matches<C: CommandFactory>(_: &ArgMatches) -> Self {
        Self
    }

    pub fn start(self) -> CommandTelemetry {
        CommandTelemetry
    }
}

pub struct CommandTelemetry;

impl CommandTelemetry {
    pub fn finish(self, _: i32) {}
}

pub struct ModuleMetadata;

impl ModuleMetadata {
    pub fn new(_: Operation, _: &str) -> Self {
        Self
    }

    pub fn start(self) -> ModuleTelemetry {
        ModuleTelemetry
    }
}

pub struct ModuleTelemetry;

impl ModuleTelemetry {
    pub fn finish(self, _: bool) {}
}
