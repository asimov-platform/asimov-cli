// This is free and unencumbered software released into the public domain.

use crate::{BoxError, StandardOptions, SysexitsError::*};
use asimov_installer::InstallOptions;
use asimov_module::{ModuleManifest, ModuleName, ReadVarError};
use color_print::ceprintln;

pub async fn install(
    module_names: Vec<ModuleName>,
    version: Option<String>,
    model_size: Option<String>,
    flags: &StandardOptions,
) -> Result<(), BoxError> {
    let registry = asimov_registry::Registry::default();
    let installer = asimov_installer::Installer::default();

    let module_names = if module_names.len() == 1 && module_names[0].as_str() == "all" {
        fetch_all_module_names()
            .await
            .map_err(|e| {
                tracing::error!("unable to fetch list of all modules: {e}");
                EX_UNAVAILABLE
            })?
            .into_iter()
            .map(ModuleName::try_from)
            .collect::<Result<Vec<_>, _>>()?
    } else {
        module_names.to_vec()
    };

    for module_name in module_names {
        if !registry
            .is_module_installed(&module_name)
            .await
            .map_err(|e| {
                tracing::error!("failed to check installation state of `{module_name}`: {e}");
                EX_IOERR
            })?
        {
            let target_version = if let Some(ref want) = version {
                want.clone()
            } else {
                installer
                    .fetch_latest_release(&module_name)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            "unable to find latest release for module `{module_name}`: {e}"
                        );
                        EX_UNAVAILABLE
                    })?
            };

            if flags.verbose > 0 {
                ceprintln!(
                    "<s,g>✓</> Found version <s>{target_version}</> for module <s>{module_name}</>."
                );
            }

            if flags.verbose > 1 {
                ceprintln!("<s,c>»</> Installing module <s>{module_name}</>...");
            }

            let install_options = InstallOptions::builder()
                .version(target_version.clone())
                .maybe_model_size(model_size.clone())
                .build();
            installer
                .install_module(&module_name, &install_options)
                .await
                .map_err(|e| {
                    tracing::error!("failed to install module `{module_name}`: {e}");
                    EX_UNAVAILABLE
                })?;

            if flags.verbose > 0 {
                ceprintln!("<s,g>✓</> Installed module <s>{module_name}</>.");
            }
        } else if flags.verbose > 0 {
            ceprintln!("<s,g>✓</> Module <s>{module_name}</> is already installed.");
        }

        if registry
            .is_module_enabled(&module_name)
            .await
            .map_err(|e| {
                tracing::error!("failed to check enabled state of `{module_name}`: {e}");
                EX_IOERR
            })?
        {
            continue;
        }

        let manifest = registry.read_manifest(&module_name).await.map_err(|e| {
            tracing::error!("failed to read module manifest for `{module_name}`: {e}");
            EX_UNAVAILABLE
        })?;

        super::config::validate_variables(&manifest.manifest)?;

        let variables = manifest
            .manifest
            .config
            .iter()
            .flat_map(|conf| conf.variables.iter());

        let mut missing_variables = Vec::new();
        for var in variables {
            if !var.is_required() {
                continue;
            }
            match manifest.manifest.variable(&var.name, None) {
                Ok(_) => (),
                Err(ReadVarError::UnconfiguredVar(_)) => {
                    missing_variables.push(var);
                },
                Err(e) => {
                    tracing::error!(
                        "failed to read configuration variable `{}` for module `{module_name}`: {e}",
                        var.name
                    );
                    return Err(EX_UNAVAILABLE.into());
                },
            }
        }

        if missing_variables.is_empty() {
            registry.enable_module(&module_name).await.map_err(|e| {
                tracing::error!("failed to enable installed module `{module_name}`: {e}");
                EX_UNAVAILABLE
            })?;
        } else {
            ceprintln!(
                "<s,y>warn:</> Module <s>{module_name}</> wasn't enabled automatically due to missing configuration."
            );
            ceprintln!("<s,dim>hint:</> Module <s>{module_name}</> requires configuration:");

            for var in missing_variables {
                let desc_suffix = if let Some(ref desc) = var.description {
                    format!(" (Description: \"{desc}\")")
                } else {
                    String::new()
                };

                ceprintln!(
                    "<s,dim>hint:</>   Missing variable: <s>{}</s>{}",
                    var.name,
                    desc_suffix
                );

                if let Some(ref env) = var.environment {
                    ceprintln!(
                        "<s,dim>hint:</>   Alternative: set environment variable: <s>{env}</>"
                    );
                }
            }

            ceprintln!(
                "<s,dim>hint:</>   To configure: <s>asimov module config setup {module_name}</s>"
            );
            ceprintln!("<s,dim>hint:</>   To enable: <s>asimov module enable {module_name}</s>");
        }
    }

    Ok(())
}

/// The manifest of the `all` meta-module, which requires every module. This
/// is GitHub's raw content origin directly, as the client won't redirect.
const ALL_MODULES_URL: &str = "https://raw.githubusercontent.com/asimov-modules/asimov-modules/master/all/.asimov/module.yaml";

/// Fetches the names of all published modules, using the environment and
/// system proxy settings for the manifest's origin, as resolved by the SDK.
pub async fn fetch_all_module_names() -> Result<Vec<String>, Box<dyn core::error::Error>> {
    let proxy = asimov_http::proxy::ProxyOptions::default();
    fetch_module_names(ALL_MODULES_URL, &proxy).await
}

/// Implements [`fetch_all_module_names`] for any URL and proxy options.
async fn fetch_module_names(
    url: &str,
    proxy: &asimov_http::proxy::ProxyOptions,
) -> Result<Vec<String>, Box<dyn core::error::Error>> {
    let url = reqwest::Url::parse(url)?;
    let proxy = asimov_http::proxy::OriginProxy::resolve(&url, proxy)?;
    let client = proxy
        .client_builder()?
        .user_agent("asimov-module-cli")
        .build()?;
    proxy.check_origin(&url)?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;

    if !response.status().is_success() {
        Err(format!(
            "HTTP status code was not successful: {0}",
            response.status()
        ))?;
    }

    let content = response
        .text()
        .await
        .inspect_err(|err| tracing::debug!(?err))?;

    let manifest: ModuleManifest = serde_yml::from_str(&content)
        .inspect_err(|err| tracing::debug!(?err, ?content))
        .map_err(|e| format!("unable to deserialize GitHub response: {e}"))?;

    Ok(manifest.requires.modules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn module_names_are_fetched_through_the_resolved_proxy() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let proxy = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 4096];
            while !request.ends_with(b"\r\n\r\n") {
                let count = socket.read(&mut buffer).await.unwrap();
                assert_ne!(count, 0, "EOF before the request ended");
                request.extend_from_slice(&buffer[..count]);
            }
            let body = "name: all\nrequires:\n  modules: [alpha, beta]\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        });
        let options = asimov_http::proxy::ProxyOptions {
            mode: asimov_http::proxy::ProxyMode::Explicit(format!("http://{address}")),
            no_proxy: Some(String::new()),
        };
        let names = fetch_module_names("http://modules.invalid/module.yaml", &options)
            .await
            .unwrap();
        assert_eq!(names, ["alpha", "beta"]);
        let request = proxy.await.unwrap();
        assert!(
            request.starts_with("GET http://modules.invalid/module.yaml HTTP/1.1\r\n"),
            "{request}"
        );
    }
}
