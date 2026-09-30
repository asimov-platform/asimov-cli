// This is free and unencumbered software released into the public domain.

use asimov_id::PublicKey;
use asimov_module::ModuleName;
pub use asimov_telemetry::Operation;
use asimov_telemetry::{Event, Outcome, Telemetry};
use clientele::crates::clap::{ArgMatches, CommandFactory};
use std::{string::String, sync::OnceLock, time::Instant, vec::Vec};

static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

pub fn initialize(public_key: &PublicKey, installed: bool) {
    if let Some(telemetry) = Telemetry::new(
        env!("ASIMOV_STATSIG_CLIENT_KEY"),
        public_key.to_string(),
        env!("CARGO_PKG_VERSION"),
    ) {
        let _ = TELEMETRY.set(telemetry);
        if installed {
            log(Event::Installed);
        }
    }
}

fn log(event: Event) {
    if let Some(telemetry) = TELEMETRY.get() {
        telemetry.log(event);
    }
}

pub struct CommandMetadata {
    command: String,
    modules: Vec<String>,
}

impl CommandMetadata {
    pub fn from_matches<C: CommandFactory>(mut matches: &ArgMatches) -> Self {
        let command = C::command();
        let mut definition = &command;
        let mut parts = Vec::new();
        let mut modules = Vec::new();
        loop {
            for id in ["name", "names", "module"] {
                if let Ok(Some(names)) = matches.try_get_many::<ModuleName>(id) {
                    modules.extend(names.map(ToString::to_string));
                }
            }
            let Some((name, submatches)) = matches.subcommand() else {
                break;
            };
            let Some(subcommand) = definition.find_subcommand(name) else {
                return Self {
                    command: "external".into(),
                    modules: Vec::new(),
                };
            };
            parts.push(subcommand.get_name());
            definition = subcommand;
            matches = submatches;
        }
        let command = match parts.join(" ").as_str() {
            "source" => "source fetch".into(),
            "source snap" => "source snap save".into(),
            "proxy" => "proxy serve".into(),
            "module config" if !modules.is_empty() => "module config show".into(),
            other => other.to_string(),
        };
        modules.sort();
        modules.dedup();
        Self { command, modules }
    }

    pub fn start(self) -> CommandTelemetry {
        let started = Instant::now();
        log(Event::CommandStarted {
            command: self.command.clone(),
            modules: self.modules.clone(),
        });
        CommandTelemetry {
            metadata: self,
            started,
        }
    }
}

pub struct CommandTelemetry {
    metadata: CommandMetadata,
    started: Instant,
}

impl CommandTelemetry {
    pub fn finish(self, exit_code: i32) {
        log(Event::CommandFinished {
            command: self.metadata.command,
            modules: self.metadata.modules,
            exit_code,
            duration: self.started.elapsed(),
        });
    }
}

pub struct ModuleMetadata {
    operation: Operation,
    module: String,
}

impl ModuleMetadata {
    pub fn new(operation: Operation, module: &str) -> Self {
        Self {
            operation,
            module: module.to_string(),
        }
    }

    pub fn start(self) -> ModuleTelemetry {
        ModuleTelemetry {
            metadata: self,
            started: Instant::now(),
        }
    }
}

pub struct ModuleTelemetry {
    metadata: ModuleMetadata,
    started: Instant,
}

impl ModuleTelemetry {
    pub fn finish(self, succeeded: bool) {
        log(Event::ModuleOperationFinished {
            operation: self.metadata.operation,
            module: self.metadata.module,
            outcome: if succeeded {
                Outcome::Success
            } else {
                Outcome::Failure
            },
            duration: self.started.elapsed(),
        });
    }
}
