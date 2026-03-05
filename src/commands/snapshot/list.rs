// This is free and unencumbered software released into the public domain.

use asimov_env::paths::asimov_root;
use asimov_registry::Registry;
use asimov_snapshot::{Options, Snapshotter};
use clientele::{StandardOptions, SysexitsError};
use color_print::cprintln;
use jiff::{Zoned, tz::TimeZone};

use crate::timestamps::format_ts_diff;

pub async fn list(flags: &StandardOptions) -> Result<(), SysexitsError> {
    let storage = asimov_snapshot::storage::Fs::for_dir(asimov_root().join("snapshots"))?;
    let ss = Snapshotter::new(Registry::default(), storage, Options::default());

    let now = Zoned::now();
    let urls = ss
        .list()
        .await
        .inspect_err(|e| tracing::error!("failed to list snapshots: {e}"))?;
    for (url, ts) in urls {
        let diff = format_ts_diff(&now, &ts.to_zoned(TimeZone::UTC))
            .expect("Unexpectedly failed to format timestamp difference");

        if flags.verbose > 0 {
            cprintln!("<s>{url}</> (last updated {diff})");
        } else {
            println!("{url}");
        }
    }
    Ok(())
}
