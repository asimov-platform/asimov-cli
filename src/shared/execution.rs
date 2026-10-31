// This is free and unencumbered software released into the public domain.

//! Local-first source routing and a common execution stream for both transports.

use crate::{BoxError, SysexitsError, SysexitsError::*, shared};
use alloc::{boxed::Box, collections::VecDeque, rc::Rc, string::String};
use asimov_keyring::Keyring;
use asimov_module::resolve::{Module, Resolver};
use asimov_patterns::Execute;
use asimov_registry::Registry;
use asimov_remote::http;
use asimov_runner::{ExecutorError, JsonlBatch, Stream, StreamExt};
use color_print::ceprintln;
use core::{future::Future, pin::Pin};
use std::io::Write;

/// The number of module subprocesses to run at once: one per CPU.
pub(crate) fn max_jobs() -> usize {
    std::thread::available_parallelism().map_or(1, core::num::NonZero::get)
}

/// Source tasks queued for output, with their context. Dropping a task's
/// handle would detach it, so on an early return these are aborted instead;
/// dropping their output streams in turn kills the module processes.
pub(crate) struct AbortOnDrop<T, O>(pub VecDeque<(T, tokio::task::JoinHandle<O>)>);

impl<T, O> Drop for AbortOnDrop<T, O> {
    fn drop(&mut self) {
        for (_, task) in &self.0 {
            task.abort();
        }
    }
}

pub(crate) enum Source {
    Local(Rc<Module>),
    Relay(Box<http::Executor>),
}

pub(crate) struct SourceResolver {
    registry: Registry,
    local: Resolver,
    enabled: Option<Resolver>,
    relay: Option<http::Executor>,
}

impl SourceResolver {
    pub async fn new(program: &str) -> Result<Self, BoxError> {
        let registry = Registry::default();
        let installed = shared::installed_modules(&registry, Some(program)).await?;
        let local = Resolver::try_from_iter(installed.iter()).map_err(|error| {
            ceprintln!("<s,r>error:</> failed to build resolver: {error}");
            EX_UNAVAILABLE
        })?;
        // Match `module resolve`: any enabled handler prevents relay fallback,
        // even if its manifest does not provide this particular program.
        let enabled = if std::env::var_os("ASIMOV_RELAY_URL").is_some() {
            let manifests = registry.enabled_modules().await.map_err(|error| {
                ceprintln!("<s,r>error:</> unable to access enabled modules: {error}");
                EX_UNAVAILABLE
            })?;
            Some(
                Resolver::try_from_iter(manifests.into_iter().map(|m| m.manifest)).map_err(
                    |error| {
                        ceprintln!("<s,r>error:</> failed to build resolver: {error}");
                        EX_UNAVAILABLE
                    },
                )?,
            )
        } else {
            None
        };
        Ok(Self {
            registry,
            local,
            enabled,
            relay: None,
        })
    }

    pub async fn resolve(&mut self, url: &str, module: Option<&str>) -> Result<Source, BoxError> {
        let invalid_url = |error| {
            ceprintln!("<s,r>error:</> unable to handle URL <s>{url}</>: {error}");
            EX_USAGE
        };
        if module.is_none()
            && let Some(enabled) = &self.enabled
            && enabled.resolve(url).map_err(invalid_url)?.is_empty()
        {
            if self.relay.is_none() {
                self.relay = Some(relay_executor()?);
            }
            return Ok(Source::Relay(Box::new(
                self.relay.as_ref().unwrap().clone(),
            )));
        }
        let modules = self.local.resolve(url).map_err(invalid_url)?;
        Ok(Source::Local(
            shared::pick_module(&self.registry, url, &modules, module).await?,
        ))
    }
}

/// Load the signing identity only when a URL actually needs the relay. Clones
/// share the HTTP pool and discovered remote key for the entire command.
fn relay_executor() -> Result<http::Executor, BoxError> {
    let url = std::env::var("ASIMOV_RELAY_URL")?;
    let secret = match std::env::var("ASIMOV_RELAY_SECRET") {
        Ok(secret) => Some(secret),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => {
            ceprintln!("<s,r>error:</> ASIMOV_RELAY_SECRET is not valid UTF-8");
            return Err(EX_CONFIG.into());
        },
    };
    let user = whoami::username().unwrap_or_else(|_| "default".into());
    // Repair any stale public-key cache before choosing the default bearer.
    let signing_key = Keyring::open()
        .and_then(|mut keyring| keyring.ensure_secret_key(&user))
        .map_err(|error| {
            ceprintln!("<s,r>error:</> failed to load local signing key: {error}");
            EX_UNAVAILABLE
        })?;
    Ok(http::Executor::new(url, secret.as_deref())?
        .with_signing_key(signing_key)
        .with_response_verification())
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ExecutionError {
    #[error(transparent)]
    Local(#[from] ExecutorError),
    #[error(transparent)]
    Remote(#[from] http::Error),
}

impl ExecutionError {
    pub fn exit_code(self) -> SysexitsError {
        match self {
            Self::Local(ExecutorError::Failure(code, _)) => code,
            _ => EX_UNAVAILABLE,
        }
    }
}

type OutputStream = Pin<Box<dyn Stream<Item = Result<JsonlBatch, ExecutionError>> + Send>>;
type Execution = Pin<Box<dyn Future<Output = Result<OutputStream, ExecutionError>> + Send>>;

/// Prepare either SDK executor without starting it until all URLs are resolved.
pub(crate) fn execute<S, E>(
    mut program: impl Execute<S, Error = E> + Send + 'static,
    url: &str,
) -> Execution
where
    S: Stream<Item = Result<JsonlBatch, E>> + Send + 'static,
    E: Into<ExecutionError> + 'static,
{
    let url = String::from(url);
    Box::pin(async move {
        let stream = loop {
            match program.execute().await.map_err(Into::into) {
                Ok(stream) => break stream,
                Err(error) => {
                    if let ExecutionError::Remote(remote) = &error
                        && let Some(delay) = remote.retry_after()
                    {
                        ceprintln!(
                            "<s,y>warning:</> relay rate limited <s>{url}</> (HTTP 429); retrying after {delay:?}..."
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                    return Err(error);
                },
            }
        };
        Ok(Box::pin(stream.map(|batch| batch.map_err(Into::into))) as OutputStream)
    })
}

/// Remote filters already ran on the server. Preserve bytes, separating final
/// unterminated records; output remains provisional until verified at EOF.
pub(crate) fn write_remote_batch(
    batch: &JsonlBatch,
    output: &mut dyn Write,
) -> std::io::Result<()> {
    for line in batch.lines() {
        output.write_all(line)?;
        if !line.ends_with(b"\n") {
            output.write_all(b"\n")?;
        }
    }
    output.flush()
}
