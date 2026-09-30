// This is free and unencumbered software released into the public domain.

use asimov_module::ModuleName;
use asimov_telemetry::{Event, Operation, Outcome, Telemetry};
use clientele::crates::clap::{ArgMatches, Command};
use std::{string::String, sync::OnceLock, time::Instant, vec::Vec};

static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

pub fn initialize(public_key: String, installed: bool) {
    let Some(key) = option_env!("ASIMOV_STATSIG_CLIENT_KEY") else {
        return;
    };
    if let Some(telemetry) = Telemetry::new(key, public_key, env!("CARGO_PKG_VERSION")) {
        let _ = TELEMETRY.set(telemetry);
        if installed {
            log(Event::Installed);
        }
    }
}

pub fn log(event: Event) {
    if let Some(telemetry) = TELEMETRY.get() {
        telemetry.log(event);
    }
}

pub struct CommandMetadata {
    pub command: String,
    pub modules: Vec<String>,
}

impl CommandMetadata {
    pub fn from_matches(mut matches: &ArgMatches, mut definition: &Command) -> Self {
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

pub fn module_operation(operation: Operation, module: String, succeeded: bool, started: Instant) {
    log(Event::ModuleOperationFinished {
        operation,
        module,
        outcome: if succeeded {
            Outcome::Success
        } else {
            Outcome::Failure
        },
        duration: started.elapsed(),
    });
}
