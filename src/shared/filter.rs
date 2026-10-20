// This is free and unencumbered software released into the public domain.

//! Shared Jev, JSON property selection, and jq processing for source records.

use crate::{BoxError, SysexitsError::*, shared};
use alloc::{collections::BTreeSet, string::String};
use asimov_patterns::FilteringOptions;
use asimov_runner::{JsonlBatch, StreamExt};
use color_print::ceprintln;
use serde_json::Value;
use std::io::Write;

pub(crate) struct OutputFilter {
    options: FilteringOptions,
    json: Option<BTreeSet<String>>,
    jq: Option<jq::JsonFilter>,
}

impl OutputFilter {
    pub fn new(options: FilteringOptions) -> Result<Self, BoxError> {
        Self::validate_json(&options, None)?;
        if options.jev.is_some() && std::env::var("TYPESAFE_API_TOKEN").is_err() {
            ceprintln!("<s,r>error:</> --jev requires TYPESAFE_API_TOKEN to be set");
            return Err(EX_CONFIG.into());
        }
        let jq = shared::compile_jq(options.jq.as_deref())?;
        let json = options
            .json
            .as_ref()
            .map(|fields| fields.iter().cloned().collect());
        Ok(Self { options, json, jq })
    }

    /// Validate selection before resolving either local modules or the relay.
    pub fn validate_json(options: &FilteringOptions, output: Option<&str>) -> Result<(), BoxError> {
        if let Some(fields) = &options.json {
            if fields.is_empty() || fields.iter().any(String::is_empty) {
                ceprintln!("<s,r>error:</> --json requires one or more nonempty property names");
                return Err(EX_USAGE.into());
            }
            if output.is_some_and(|format| format != "jsonl") {
                ceprintln!("<s,r>error:</> --json requires JSONL output (--output=jsonl)");
                return Err(EX_USAGE.into());
            }
        }
        Ok(())
    }

    pub async fn write_batch(
        &self,
        batch: &JsonlBatch,
        url: &str,
        output: &mut dyn Write,
    ) -> Result<(), BoxError> {
        self.write_batch_with(batch, url, output, |filter, inputs| {
            shared::filter_jev_batch(filter, inputs)
        })
        .await
    }

    // Keep the evaluator injectable so ordering, grouping, and failures can be
    // tested without credentials or calls to the live Jev service.
    async fn write_batch_with<'a, S: futures_lite::Stream<Item = shared::JevLine>>(
        &'a self,
        batch: &'a JsonlBatch,
        url: &str,
        output: &mut dyn Write,
        mut evaluate: impl FnMut(&'a str, alloc::vec::Vec<&'a [u8]>) -> Result<S, BoxError>,
    ) -> Result<(), BoxError> {
        let mut write_line = |line: &[u8]| -> Result<(), BoxError> {
            let values = if let Some(fields) = &self.json {
                let mut value = serde_json::from_slice(line).map_err(|error| {
                    ceprintln!(
                        "<s,r>error:</> JSON property selection failed for <s>{url}</>: {error}"
                    );
                    EX_DATAERR
                })?;
                select_properties(&mut value, fields).map_err(|error| {
                    ceprintln!(
                        "<s,r>error:</> JSON property selection failed for <s>{url}</>: {error}"
                    );
                    EX_DATAERR
                })?;
                Some(if let Some(filter) = &self.jq {
                    shared::filter_jq_value(filter, value)
                } else {
                    Ok(alloc::vec![value])
                })
            } else {
                self.jq
                    .as_ref()
                    .map(|filter| shared::filter_jq(filter, line))
            };
            if let Some(values) = values {
                for value in values.map_err(|error| {
                    ceprintln!("<s,r>error:</> jq filtering failed for <s>{url}</>: {error}");
                    EX_DATAERR
                })? {
                    writeln!(output, "{value}")?;
                }
            } else {
                output.write_all(line)?;
            }
            if self.options.jev.is_some() {
                output.flush()?;
            }
            Ok(())
        };

        if let Some(filter) = self.options.jev.as_deref() {
            let mut lines = batch.lines();
            while lines.len() > 0 {
                let matches = evaluate(
                    filter,
                    lines.by_ref().take(shared::JEV_BATCH_SIZE).collect(),
                )?;
                futures_lite::pin!(matches);
                while let Some(line) = matches.next().await {
                    write_line(&line?)?;
                }
            }
        } else {
            for line in batch.lines() {
                write_line(line)?;
            }
        }
        output.flush()?;
        Ok(())
    }
}

fn select_properties(value: &mut Value, fields: &BTreeSet<String>) -> Result<(), &'static str> {
    let select_object = |value: &mut Value| {
        let object = value
            .as_object_mut()
            .ok_or("expected a JSON object or an array of objects")?;
        object.retain(|name, _| fields.contains(name));
        Ok(())
    };
    match value {
        Value::Array(values) => values.iter_mut().try_for_each(select_object),
        _ => select_object(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, vec::Vec};
    use asimov_runner::JsonlLine;
    use serde_json::{Value, json};

    #[tokio::test]
    async fn jev_sees_original_records_before_jq_and_uses_bounded_groups() {
        let batch = JsonlBatch::new(
            (0..45)
                .map(|id| JsonlLine::owned(format!("{{\"id\":{id}}}\n").into_bytes()).unwrap())
                .collect(),
        );
        let filter = OutputFilter {
            options: FilteringOptions::builder().jev("Keep even IDs").build(),
            json: None,
            jq: shared::compile_jq(Some(".id, (.id + 100)")).unwrap(),
        };
        let mut output = Vec::new();
        let mut groups = Vec::new();
        filter
            .write_batch_with(&batch, "test:", &mut output, |expression, inputs| {
                assert_eq!(expression, "Keep even IDs");
                groups.push(inputs.len());
                let matches: Vec<_> = inputs
                    .into_iter()
                    .filter(|line| {
                        let input: Value = serde_json::from_slice(line).unwrap();
                        input["id"]
                            .as_u64()
                            .expect("Jev must receive original objects")
                            .is_multiple_of(2)
                    })
                    .map(|line| Ok(line.to_vec()))
                    .collect();
                Ok(futures_lite::stream::iter(matches))
            })
            .await
            .unwrap();
        assert_eq!(groups, [20, 20, 5]);
        let actual: Vec<Value> = serde_json::Deserializer::from_slice(&output)
            .into_iter()
            .map(Result::unwrap)
            .collect();
        let expected: Vec<_> = (0..45)
            .step_by(2)
            .flat_map(|id| [json!(id), json!(id + 100)])
            .collect();
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn jev_sees_original_records_and_jq_sees_selected_properties() {
        let options = FilteringOptions::builder()
            .json(alloc::vec!["name".into()])
            .jq(r#"if has("keep") then error("unselected property") else {label: .name}, .name end"#)
            .build();
        let mut filter = OutputFilter::new(options).unwrap();
        // Use the injected evaluator without requiring live Jev credentials.
        filter.options.jev = Some("Keep matching records".into());
        let batch = JsonlBatch::from_bytes(asimov_runner::Bytes::from_static(
            b"{\"keep\":true,\"name\":\"Ada\"}\n{\"keep\":false,\"name\":\"Grace\"}\n",
        ));
        let mut output = Vec::new();
        filter
            .write_batch_with(&batch, "test:", &mut output, |_, inputs| {
                let matches: Vec<_> = inputs
                    .into_iter()
                    .filter(|line| {
                        let value: Value = serde_json::from_slice(line).unwrap();
                        value["keep"]
                            .as_bool()
                            .expect("Jev sees original properties")
                    })
                    .map(|line| Ok(line.to_vec()))
                    .collect();
                Ok(futures_lite::stream::iter(matches))
            })
            .await
            .unwrap();
        assert_eq!(output, b"{\"label\":\"Ada\"}\n\"Ada\"\n");
    }

    #[test]
    fn programmatic_json_selection_requires_nonempty_names() {
        for fields in [alloc::vec![], alloc::vec![String::new()]] {
            let options = FilteringOptions::builder().json(fields).build();
            let error = OutputFilter::new(options).err().unwrap();
            assert_eq!(error.downcast_ref(), Some(&EX_USAGE));
        }
    }

    #[tokio::test]
    async fn jev_failures_stop_output_without_hiding_the_error() {
        let batch = JsonlBatch::from_bytes(asimov_runner::Bytes::from_static(b"{}\n"));
        let filter = OutputFilter {
            options: FilteringOptions::builder().jev("anything").build(),
            json: None,
            jq: None,
        };
        let mut output = Vec::new();
        let result = filter
            .write_batch_with(&batch, "test:", &mut output, |_, _| {
                Ok(futures_lite::stream::iter([
                    Err::<Vec<u8>, BoxError>(EX_UNAVAILABLE.into()),
                    Ok(b"{}\n".to_vec()),
                ]))
            })
            .await;
        assert_eq!(result.unwrap_err().downcast_ref(), Some(&EX_UNAVAILABLE));
        assert!(output.is_empty());
    }
}
