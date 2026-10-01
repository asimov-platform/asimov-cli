// This is free and unencumbered software released into the public domain.

use crate::{BoxError, SysexitsError::*, telemetry};
use clientele::{StandardOptions, crates::clap::Subcommand};
use color_print::{ceprintln, cprintln};

#[derive(Debug, Subcommand)]
pub enum ConfigureCommand {
    /// Enable or disable usage telemetry.
    ///
    /// Telemetry is also disabled when ASIMOV_TELEMETRY is set to 0 or false.
    #[clap(subcommand)]
    Telemetry(TelemetryCommand),
}

#[derive(Clone, Copy, Debug, Subcommand)]
pub enum TelemetryCommand {
    /// Enable usage telemetry.
    Enable {},

    /// Disable usage telemetry and discard any unsent events.
    Disable {},
}

impl ConfigureCommand {
    pub async fn run(self, flags: &StandardOptions) -> Result<(), BoxError> {
        let ConfigureCommand::Telemetry(command) = self;
        let result = match command {
            TelemetryCommand::Enable {} => telemetry::enable(),
            TelemetryCommand::Disable {} => telemetry::disable(),
        };
        result.map_err(|e| {
            ceprintln!("<s,r>error:</> failed to configure telemetry: {e}");
            EX_IOERR
        })?;
        if flags.verbose > 0 {
            match command {
                TelemetryCommand::Enable {} => cprintln!("<s,g>✓</> Enabled telemetry."),
                TelemetryCommand::Disable {} => cprintln!("<s,g>✓</> Disabled telemetry."),
            }
        }
        Ok(())
    }
}
