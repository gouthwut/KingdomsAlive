//! Authorization and replay guard for the explicitly enabled, one-shot M2 pilot.
//! A claim is not a transaction: an uncertain physical outcome must never be replayed.
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::num::NonZeroU128;
use std::path::Path;

use crate::settlement::Settlement;

pub const REQUEST: &str = "kingdoms_alive_delivery.json";
pub const CLAIM: &str = "kingdoms_alive_delivery.claim";

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub resident: NonZeroU128,
    pub chest: [i32; 3],
    pub approach: [i32; 3],
    pub chest_state: String,
    pub chest_name: String,
}

impl Request {
    pub fn open(dir: &Path, settlement: &Settlement) -> io::Result<Option<Self>> {
        let bytes = match fs::read(dir.join(REQUEST)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let request: Self = serde_json::from_slice(&bytes)?;
        let [x, y, z] = request.chest;
        let [ax, ay, az] = request.approach;
        let territory = &settlement.territory;
        if !settlement.residents.contains(&request.resident)
            || !territory.contains(&territory.dimension, x, z)
            || !territory.contains(&territory.dimension, ax, az)
            || y != ay
            || u64::from(x.abs_diff(ax)) + u64::from(z.abs_diff(az)) != 1
            || request.chest_name.trim().is_empty()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid delivery request",
            ));
        }
        Ok(Some(request))
    }

    /// Deleting or changing approval revokes this in-memory task, including while walking.
    pub fn still_approved(&self, dir: &Path, settlement: &Settlement) -> bool {
        matches!(Self::open(dir, settlement), Ok(Some(current)) if current == *self)
    }
}

/// Create before touching either inventory. Existing or partially written claims block retries.
pub fn claim(path: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(
        b"Delivery attempted. Reconcile physical inventories before another attempt.\n",
    )?;
    file.sync_all()
}
