// This is free and unencumbered software released into the public domain.

use crate::{BoxError, Result};
use asimov_module::{ModuleManifest, resolve::Module};
use clientele::{Subcommand, SubcommandsProvider, SysexitsError::*};
use color_print::{ceprintln, cstr};
use std::io::Write;
use std::pin::Pin;
use std::{rc::Rc, sync::LazyLock};

/// Returns a lazily initialized HTTP client with a shared connection pool.
///
/// Clones are cheap and reuse the same underlying client and connection pool.
pub fn http_client() -> reqwest::Client {
    // let http_client = reqwest::Client::builder()
    //     .connect_timeout(std::time::Duration::from_secs(10))
    //     .timeout(std::time::Duration::from_secs(120))
    //     .build()?;
    static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);
    CLIENT.clone()
}

/// Locates the given subcommand or prints an error.
pub fn locate_subcommand(name: &str) -> Result<Subcommand> {
    match SubcommandsProvider::find("asimov-", name) {
        Some(cmd) => Ok(cmd),
        None => {
            eprintln!("asimov: command not found: asimov-{}", name);
            Err(EX_UNAVAILABLE)
        },
    }
}

const NO_MODULES_FOUND_HINT: &str = cstr!(
    r#"<s,dim>hint:</> There appears to be no installed modules.
<s,dim>hint:</> Modules may be discovered either on the site <u>https://asimov.directory/modules</>
<s,dim>hint:</> or on the GitHub organization <u>https://github.com/asimov-modules</>
<s,dim>hint:</> and installed with <s>asimov module install <<module>></>"#
);

pub async fn installed_modules(
    registry: &asimov_registry::Registry,
    filter: Option<&str>,
) -> Result<Vec<ModuleManifest>> {
    let modules = registry
        .installed_modules()
        .await
        .map_err(|e| {
            ceprintln!("<s,r>error:</> unable to access installed modules: {e}");
            match e {
                asimov_registry::error::InstalledModulesError::DirIo(_, err)
                    if err.kind() == std::io::ErrorKind::NotFound =>
                {
                    ceprintln!("{NO_MODULES_FOUND_HINT}");
                },
                _ => (),
            }
            EX_UNAVAILABLE
        })?
        .into_iter()
        .map(|manifest| manifest.manifest)
        .filter(|manifest| {
            if let Some(filter) = filter {
                manifest
                    .provides
                    .programs
                    .iter()
                    .any(|program| program.split('-').next_back().is_some_and(|p| p == filter))
            } else {
                true
            }
        })
        .collect();

    Ok(modules)
}

pub async fn pick_module(
    registry: &asimov_registry::Registry,
    url: impl AsRef<str>,
    modules: &[Rc<Module>],
    filter: Option<&str>,
) -> Result<Rc<Module>> {
    let url = url.as_ref();

    if let Some(filter) = filter {
        let module = modules.iter().find(|m| m.name == filter).ok_or_else(|| {
                ceprintln!("<s,r>error:</> failed to find a module named `{filter}` that supports handling the URL <s>{url}</>");
                EX_SOFTWARE
            })?;

        let module_name = module.name.parse().map_err(|e| {
            ceprintln!("<s,r>error:</> {e}");
            EX_DATAERR
        })?;

        if !registry
            .is_module_enabled(&module_name)
            .await
            .map_err(|e| {
                ceprintln!(
                    "<s,r>error:</> error while checking whether module <s>{}</> is enabled: {e}",
                    module.name
                );
                EX_IOERR
            })?
        {
            ceprintln!(
                "<s,r>error:</> module <s>{}</> is not enabled.",
                module.name
            );
            ceprintln!(
                "<s,dim>hint:</> It can be enabled with: <s>asimov module enable {}</>",
                module.name
            );
            Err(EX_UNAVAILABLE)
        } else {
            Ok(module.clone())
        }
    } else {
        let mut iter = modules.iter();
        loop {
            let module = iter.next().ok_or_else(|| {
                    ceprintln!(
                        "<s,r>error:</> failed to find a module to handle the URL <s>{url}</>"
                    );
                    let module_count = modules.len();
                    if module_count > 0 {
                        if module_count == 1 {
                            ceprintln!("<s,dim>hint:</> Found <s>{module_count}</> installed module that could handle this URL but is disabled.");
                        } else {
                            ceprintln!("<s,dim>hint:</> Found <s>{module_count}</> installed modules that could handle this URL but are disabled.");
                        }
                        ceprintln!("<s,dim>hint:</> A module can be enabled with: <s>asimov module enable <<module>></>");
                        ceprintln!("<s,dim>hint:</> Available modules:");
                        for module in modules {
                            ceprintln!("<s,dim>hint:</>\t<s>{}</>", module.name);
                        }
                    }
                    EX_UNAVAILABLE
                })?;

            let module_name = module.name.parse().map_err(|e| {
                ceprintln!("<s,r>error:</> {e}");
                EX_DATAERR
            })?;

            if registry
                .is_module_enabled(&module_name)
                .await
                .map_err(|e| {
                    ceprintln!(
                        "<s,r>error:</> error while checking whether module <s>{}</> is enabled: {e}",
                        module.name
                    );
                    EX_IOERR
                })?
            {
                return Ok(module.clone());
            }
        }
    }
}

/// Maximum number of lines in a Jev filtering group.
pub const JEV_BATCH_SIZE: usize = 20;

pub const JEV_MATCH_THRESHOLD: f64 = 0.80;

/// Filters a Jev group, yielding passing lines in completion order.
///
/// Pass at most `JEV_BATCH_SIZE` lines and drain the stream before submitting
/// the next group. Currently each line is sent as an individual HTTP request;
/// upstream batch support can replace that implementation behind this API.
/// Dropping the stream aborts any outstanding requests in the group.
pub fn filter_jev_batch(
    filter: impl AsRef<str>,
    inputs: impl IntoIterator<Item = impl AsRef<[u8]>>,
) -> Result<Pin<Box<impl futures_lite::Stream<Item = Result<Vec<u8>, BoxError>>>>, BoxError> {
    let Ok(api_token) = std::env::var("TYPESAFE_API_TOKEN") else {
        return Err(EX_CONFIG)?;
    };
    let filter: String = json_escape::escape_str(filter.as_ref()).collect();
    let inputs: Vec<Vec<u8>> = inputs
        .into_iter()
        .map(|input| input.as_ref().to_vec())
        .collect();
    let moved_inputs = inputs.clone();
    Ok(Box::pin(async_stream::stream! {
        let response = post_typesafe(&api_token, move |out| {
            write!(out, r#"{{"#)?;
            write!(out, r#""model":"jev-latest","#)?;
            write!(out, r#""state":{{"#)?;
            write!(out, r#""rubric":"{}","#, filter)?;
            write!(out, r#""inputs":["#)?;
            for (i, input) in moved_inputs.iter().enumerate() {
                if i > 0 {
                    out.write(b",")?;
                }
                out.write_all(input.trim_ascii())?;
            }
            write!(out, r#"]"#)?;
            write!(out, r#"}},"#)?; // "state":{...},
            write!(out, r#""questions":{{"#)?;
            for i in 0..moved_inputs.len() {
                if i > 0 {
                    out.write(b",")?;
                }
                write!(out, r#""q{i}":{{"type":"noul","instructions":"Does `rubric` describe `inputs[{i}]`?"}}"#)?;
            }
            write!(out, r#"}}"#)?; // "questions":{...},
            write!(out, r#"}}"#)?;
            Ok(())
        }).await?;
        let output: JevResponse = response.json().await?;
        for (i, answer) in output.answers.0.into_iter().enumerate() {
            if answer.noul > JEV_MATCH_THRESHOLD {
                yield Ok(inputs[i].clone());
            }
        }
    }))
}

/// Posts caller-written JSON to TypeSafe's API without buffering the entire body.
///
/// `write_json` runs on a blocking worker and must write a complete JSON value.
/// It can use `write!`, `serde_json::to_writer`, or
/// `json_streaming::blocking::JsonWriter`. Captured data must be owned
/// (`Send + 'static`). Upload buffering is bounded, with backpressure applied
/// to the writer. The response body is returned unread; non-success HTTP status
/// codes and errors generating the request body are reported as errors.
pub async fn post_typesafe<F>(api_token: &str, write_json: F) -> Result<reqwest::Response, BoxError>
where
    F: FnOnce(&mut dyn Write) -> std::io::Result<()> + Send + 'static,
{
    struct BodyWriter(tokio::sync::mpsc::Sender<Vec<u8>>);

    impl Write for BodyWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.is_empty() {
                return Ok(0);
            }
            let len = bytes.len().min(16 * 1024);
            self.0.blocking_send(bytes[..len].to_vec()).map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "request body closed")
            })?;
            Ok(len)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let (sender, receiver) = tokio::sync::mpsc::channel(4);
    let producer = tokio::task::spawn_blocking(move || {
        let mut writer = std::io::BufWriter::with_capacity(16 * 1024, BodyWriter(sender));
        write_json(&mut writer)?;
        writer.flush()
    });
    let body = futures_lite::stream::unfold(
        (receiver, Some(producer)),
        |(mut receiver, mut producer)| async move {
            if let Some(chunk) = receiver.recv().await {
                return Some((Ok(chunk), (receiver, producer)));
            }
            // Check the worker before signaling EOF, so generation errors (and
            // panics) fail the upload rather than silently truncating the JSON.
            let result = producer
                .take()?
                .await
                .unwrap_or_else(|err| Err(std::io::Error::other(err)));
            result.err().map(|err| (Err(err), (receiver, producer)))
        },
    );

    Ok(http_client()
        .post("https://api.typesafe.ai/v1/systemone")
        .bearer_auth(api_token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(reqwest::Body::wrap_stream(body))
        .send()
        .await?
        .error_for_status()?)
}

#[derive(Debug, serde::Deserialize)]
pub struct JevResponse {
    pub model: String,
    pub answers: JevAnswers,
    pub usage: JevUsage,
}

#[derive(Debug)]
pub struct JevAnswers(pub Vec<JevOutput>);

impl<'de> serde::Deserialize<'de> for JevAnswers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // 1. Parse JSON map into a BTreeMap
        let map = std::collections::BTreeMap::<String, JevOutput>::deserialize(deserializer)?;

        // 2. Filter keys starting with "q", extract integer index
        let mut entries: Vec<(usize, JevOutput)> = map
            .into_iter()
            .filter_map(|(key, val)| {
                key.strip_prefix('q')
                    .and_then(|idx_str| idx_str.parse::<usize>().ok())
                    .map(|idx| (idx, val))
            })
            .collect();

        // 3. Sort numerically by index (ensuring q2 comes before q10)
        entries.sort_by_key(|(idx, _)| *idx);

        Ok(Self(entries.into_iter().map(|(_, val)| val).collect()))
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct JevOutput {
    #[serde(rename = "type")]
    pub r#type: String,
    pub noul: f64,
}

#[derive(Debug, serde::Deserialize)]
pub struct JevUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

pub fn compile_jq(expression: Option<&str>) -> Result<Option<jq::JsonFilter>> {
    expression
        .map(|expression| {
            expression.parse::<jq::JsonFilter>().map_err(|e| {
                ceprintln!("<s,r>error:</> invalid jq expression: {e}");
                EX_DATAERR
            })
        })
        .transpose()
}

pub fn filter_jq(
    filter: &jq::JsonFilter,
    input: impl AsRef<[u8]>,
) -> std::result::Result<Vec<serde_json::Value>, BoxError> {
    serde_json::Deserializer::from_slice(input.as_ref())
        .into_iter::<serde_json::Value>()
        .filter_map(|value| {
            let value = match value {
                Ok(value) => value,
                Err(e) => return Some(Err(e.into())),
            };

            match filter.filter_json(value) {
                Ok(value) => Some(Ok(value)),
                Err(jq::JsonFilterError::NoOutput) => None,
                Err(e) => Some(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )
                .into())),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_json_values() {
        let filter = ".name".parse::<jq::JsonFilter>().unwrap();
        let values = filter_jq(
            &filter,
            br#"{"name":"first"}
{"name":"second"}"#,
        )
        .unwrap();

        assert_eq!(
            values,
            [serde_json::json!("first"), serde_json::json!("second")]
        );
    }

    #[test]
    fn suppresses_values_without_jq_output() {
        let filter = "select(.keep)".parse::<jq::JsonFilter>().unwrap();
        let values = filter_jq(
            &filter,
            br#"{"name":"first","keep":true}
{"name":"second","keep":false}"#,
        )
        .unwrap();

        assert_eq!(values, [serde_json::json!({"name": "first", "keep": true})]);
    }
}
