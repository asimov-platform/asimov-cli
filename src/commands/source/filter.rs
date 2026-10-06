// This is free and unencumbered software released into the public domain.

//! Shared Jev-then-jq processing for fetched and listed records.

use crate::{BoxError, SysexitsError::*, shared};
use asimov_patterns::FilteringOptions;
use asimov_runner::{JsonlBatch, StreamExt};
use color_print::ceprintln;
use std::io::Write;

pub(super) struct OutputFilter {
    options: FilteringOptions,
    jq: Option<jq::JsonFilter>,
}

impl OutputFilter {
    pub fn new(options: FilteringOptions) -> Result<Self, BoxError> {
        if options.jev.is_some() && std::env::var("TYPESAFE_API_TOKEN").is_err() {
            ceprintln!("<s,r>error:</> --jev requires TYPESAFE_API_TOKEN to be set");
            return Err(EX_CONFIG.into());
        }
        let jq = shared::compile_jq(options.jq.as_deref())?;
        Ok(Self { options, jq })
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
            if let Some(filter) = self.jq.as_ref() {
                for value in shared::filter_jq(filter, line).map_err(|error| {
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
    async fn jev_failures_stop_output_without_hiding_the_error() {
        let batch = JsonlBatch::from_bytes(asimov_runner::Bytes::from_static(b"{}\n"));
        let filter = OutputFilter {
            options: FilteringOptions::builder().jev("anything").build(),
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
