// This is free and unencumbered software released into the public domain.

use crate::{BoxError, Result};
use asimov_module::{ModuleManifest, resolve::Module};
use clientele::{Subcommand, SubcommandsProvider, SysexitsError::*};
use color_print::{ceprintln, cstr};
use std::io::Write;
use std::pin::Pin;
use std::{rc::Rc, sync::LazyLock};

#[cfg_attr(not(feature = "telemetry"), path = "shared/telemetry_disabled.rs")]
pub mod telemetry;

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

/// Filters a Jev group, yielding passing lines in input order.
///
/// Pass at most `JEV_BATCH_SIZE` lines and drain the stream before submitting
/// the next group. Each group is sent in one request, and all answer IDs,
/// types, and scores are validated before any line is emitted.
/// Every input must contain one complete JSON value; invalid inputs fail
/// before a request is created.
/// Dropping the stream cancels the outstanding request.
pub fn filter_jev_batch(
    filter: impl AsRef<str>,
    inputs: impl IntoIterator<Item = impl AsRef<[u8]>>,
) -> Result<Pin<Box<impl futures_lite::Stream<Item = Result<Vec<u8>, BoxError>>>>, BoxError> {
    let inputs = collect_jev_inputs(inputs)?;
    let Ok(api_token) = std::env::var("TYPESAFE_API_TOKEN") else {
        return Err(EX_CONFIG)?;
    };
    let filter: String = json_escape::escape_str(filter.as_ref()).collect();
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
                    out.write_all(b",")?;
                }
                out.write_all(input.trim_ascii())?;
            }
            write!(out, r#"]"#)?;
            write!(out, r#"}},"#)?; // "state":{...},
            write!(out, r#""questions":{{"#)?;
            for i in 0..moved_inputs.len() {
                if i > 0 {
                    out.write_all(b",")?;
                }
                write!(out, r#""q{i}":{{"type":"noul","instructions":"Does `rubric` describe `inputs[{i}]`?"}}"#)?;
            }
            write!(out, r#"}}"#)?; // "questions":{...},
            write!(out, r#"}}"#)?;
            Ok(())
        }).await?;
        let output: JevResponse = response.json().await?;
        for i in output.answers.matching_indices(inputs.len())? {
            yield Ok(inputs[i].clone());
        }
    }))
}

fn collect_jev_inputs(
    inputs: impl IntoIterator<Item = impl AsRef<[u8]>>,
) -> Result<Vec<Vec<u8>>, BoxError> {
    inputs
        .into_iter()
        .enumerate()
        .map(|(index, input)| {
            let input = input.as_ref();
            serde_json::from_slice::<serde_json::Value>(input).map_err(|error| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid JSON in Jev input {}: {error}", index + 1),
                )
            })?;
            Ok(input.to_vec())
        })
        .collect()
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
pub struct JevAnswers(pub std::collections::BTreeMap<usize, JevOutput>);

impl JevAnswers {
    /// Validates the entire batch before selecting any input indices.
    fn matching_indices(&self, input_count: usize) -> std::io::Result<Vec<usize>> {
        let invalid = |message| std::io::Error::new(std::io::ErrorKind::InvalidData, message);
        if self.0.len() != input_count {
            return Err(invalid("Jev answer count does not match input count"));
        }
        let mut matches = Vec::new();
        for (expected, (&id, answer)) in self.0.iter().enumerate() {
            if id != expected {
                return Err(invalid(
                    "Jev answer IDs must match q0 through qN for the inputs",
                ));
            }
            if answer.r#type != "noul" {
                return Err(invalid("Jev answer type must be noul"));
            }
            if !answer.noul.is_finite() || !(0.0..=1.0).contains(&answer.noul) {
                return Err(invalid(
                    "Jev answer score must be finite and between 0 and 1",
                ));
            }
            if answer.noul > JEV_MATCH_THRESHOLD {
                matches.push(id);
            }
        }
        Ok(matches)
    }
}

impl<'de> serde::Deserialize<'de> for JevAnswers {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct AnswersVisitor;

        impl<'de> serde::de::Visitor<'de> for AnswersVisitor {
            type Value = JevAnswers;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a map of unique Jev question IDs to answers")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::MapAccess<'de>,
            {
                use serde::de::Error;

                let mut answers = std::collections::BTreeMap::new();
                while let Some((key, answer)) = map.next_entry::<String, JevOutput>()? {
                    let id = key
                        .strip_prefix('q')
                        .and_then(|digits| digits.parse::<usize>().ok())
                        .filter(|id| key == format!("q{id}"))
                        .ok_or_else(|| M::Error::custom("invalid Jev question ID"))?;
                    if answers.insert(id, answer).is_some() {
                        return Err(M::Error::custom("duplicate Jev question ID"));
                    }
                }
                Ok(JevAnswers(answers))
            }
        }

        deserializer.deserialize_map(AnswersVisitor)
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

/// Collects every jq result in input order, failing on any JSON or jq error.
pub fn filter_jq(
    filter: &jq::JsonFilter,
    input: impl AsRef<[u8]>,
) -> std::result::Result<Vec<serde_json::Value>, BoxError> {
    let mut values = Vec::new();
    for input in
        serde_json::Deserializer::from_slice(input.as_ref()).into_iter::<serde_json::Value>()
    {
        values.extend(
            filter
                .filter_json_all(input?)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?,
        );
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jev_input_validation_preserves_original_records() {
        let inputs = [
            " {\"text\":\"hello\"}\r\n",
            "[1,2]",
            "null",
            "true",
            "42",
            "\"value\"",
        ];
        let collected = collect_jev_inputs(inputs).unwrap();
        for (actual, expected) in collected.iter().zip(inputs) {
            assert_eq!(actual, expected.as_bytes());
        }
    }

    #[test]
    fn invalid_jev_batches_fail_before_creating_a_stream() {
        for input in [
            b"".as_slice(),
            b"{broken}",
            b"1 2",
            b"null, false",
            b"\xff",
            b"\"\xff\"",
            b"[1,",
        ] {
            let Err(error) = filter_jev_batch("anything", [b"{}".as_slice(), input]) else {
                panic!("invalid JSON must fail before creating the request stream");
            };
            let error = error.downcast_ref::<std::io::Error>().unwrap();
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            assert!(error.to_string().contains("Jev input 2"));
        }
    }

    #[test]
    fn jev_answers_select_inputs_by_numeric_id() {
        let answers = (0..12)
            .rev()
            .map(|id| {
                let score = if id == 2 || id == 10 { 1 } else { 0 };
                format!(r#""q{id}":{{"type":"noul","noul":{score}}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        let answers: JevAnswers = serde_json::from_str(&format!("{{{answers}}}")).unwrap();
        assert_eq!(answers.matching_indices(12).unwrap(), [2, 10]);
    }

    #[test]
    fn jev_answers_reject_missing_and_extra_ids() {
        for (json, count) in [
            (r#"{}"#, 1),
            (r#"{"q1":{"type":"noul","noul":1}}"#, 2),
            (r#"{"q1":{"type":"noul","noul":1}}"#, 1),
            (r#"{"q0":{"type":"noul","noul":1}}"#, 0),
            (
                r#"{"q0":{"type":"noul","noul":1},"q2":{"type":"noul","noul":1}}"#,
                2,
            ),
            (
                r#"{"q0":{"type":"noul","noul":1},"q1":{"type":"noul","noul":1}}"#,
                1,
            ),
        ] {
            let answers: JevAnswers = serde_json::from_str(json).unwrap();
            assert!(
                answers.matching_indices(count).is_err(),
                "{json}, count={count}"
            );
        }
    }

    #[test]
    fn jev_answers_reject_duplicate_and_malformed_ids() {
        assert!(
            serde_json::from_str::<JevAnswers>(
                r#"{"q0":{"type":"noul","noul":0},"q0":{"type":"noul","noul":1}}"#
            )
            .is_err()
        );
        for id in [
            "",
            "q",
            "Q0",
            "q-1",
            "q+1",
            "q01",
            "q 1",
            "other",
            "q184467440737095516160",
        ] {
            let json = serde_json::json!({id: {"type": "noul", "noul": 1}});
            assert!(serde_json::from_value::<JevAnswers>(json).is_err(), "{id}");
        }
    }

    #[test]
    fn jev_answers_validate_all_types_and_scores_before_selecting() {
        for invalid in [
            serde_json::json!({"type": "other", "noul": 1}),
            serde_json::json!({"type": "noul", "noul": -0.1}),
            serde_json::json!({"type": "noul", "noul": 1.1}),
        ] {
            let answers: JevAnswers = serde_json::from_value(serde_json::json!({
                "q0": {"type": "noul", "noul": 1},
                "q1": invalid,
            }))
            .unwrap();
            assert!(answers.matching_indices(2).is_err());
        }
        for score in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let answers = JevAnswers(
                [(
                    0,
                    JevOutput {
                        r#type: "noul".into(),
                        noul: score,
                    },
                )]
                .into(),
            );
            assert!(answers.matching_indices(1).is_err());
        }
        for invalid in [
            serde_json::json!({"noul": 1}),
            serde_json::json!({"type": "noul"}),
            serde_json::json!({"type": "noul", "noul": "1"}),
            serde_json::json!({"type": "noul", "noul": null}),
            serde_json::json!({"type": "noul", "noul": true}),
        ] {
            assert!(
                serde_json::from_value::<JevAnswers>(serde_json::json!({"q0": invalid})).is_err()
            );
        }
    }

    #[test]
    fn jev_answers_preserve_the_strict_match_threshold() {
        let answers: JevAnswers = serde_json::from_str(
            r#"{
            "q0": {"type":"noul","noul":0},
            "q1": {"type":"noul","noul":0.8},
            "q2": {"type":"noul","noul":0.81},
            "q3": {"type":"noul","noul":1}
        }"#,
        )
        .unwrap();
        assert_eq!(answers.matching_indices(4).unwrap(), [2, 3]);
        let empty: JevAnswers = serde_json::from_str("{}").unwrap();
        assert!(empty.matching_indices(0).unwrap().is_empty());
    }

    #[test]
    fn filters_json_values() {
        let filter = compile_jq(Some(".name")).unwrap().unwrap();
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
        let filter = compile_jq(Some("select(.keep)")).unwrap().unwrap();
        let values = filter_jq(
            &filter,
            br#"{"name":"first","keep":true}
{"name":"second","keep":false}"#,
        )
        .unwrap();

        assert_eq!(values, [serde_json::json!({"name": "first", "keep": true})]);
    }

    #[test]
    fn preserves_every_jq_result_in_order() {
        let filter = compile_jq(Some(".[]")).unwrap().unwrap();
        let values = filter_jq(&filter, b"[1,2]\n[]\n[3,4]").unwrap();
        assert_eq!(values, vec![1, 2, 3, 4]);
    }

    #[test]
    fn reports_jq_errors_after_a_result() {
        let filter = compile_jq(Some(r#".[], error("late failure")"#))
            .unwrap()
            .unwrap();
        let error = filter_jq(&filter, b"[1,2]").unwrap_err();
        assert!(error.to_string().contains("late failure"));
    }

    #[test]
    fn rejects_invalid_jq_and_json() {
        assert!(compile_jq(Some(".[")).is_err());
        assert!(compile_jq(Some("undefined_function")).is_err());
        assert!(compile_jq(None).unwrap().is_none());
        let filter = compile_jq(Some(".")).unwrap().unwrap();
        assert!(filter_jq(&filter, b"1\n{invalid}").is_err());
    }
}
