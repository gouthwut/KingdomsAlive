use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::settlement::Settlement;

const SAVE_NAME: &str = "kingdoms_alive.json";
const SCHEMA: u32 = 1;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Deserialize)]
struct Version {
    schema: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot<S> {
    schema: u32,
    settlement: S,
}

pub fn load(world_dir: &Path) -> io::Result<Option<Settlement>> {
    let bytes = match fs::read(world_dir.join(SAVE_NAME)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let version: Version = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if version.schema != SCHEMA {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unsupported settlement schema {}", version.schema),
        ));
    }
    let snapshot: Snapshot<Settlement> = serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(snapshot.settlement))
}

/// The authoritative world thread owns the snapshot and serializes its writes.
pub fn save(world_dir: &Path, settlement: &Settlement) -> io::Result<()> {
    // Preserve an unreadable or newer snapshot for recovery.
    load(world_dir)?;
    let snapshot = Snapshot {
        schema: SCHEMA,
        settlement,
    };
    let bytes = serde_json::to_vec_pretty(&snapshot).map_err(io::Error::other)?;
    let (temp_path, mut file) = create_temp(world_dir)?;
    let written = file.write_all(&bytes).and_then(|()| file.sync_all());
    drop(file);

    let result = written.and_then(|()| fs::rename(&temp_path, world_dir.join(SAVE_NAME)));
    if result.is_err() {
        let _ = fs::remove_file(temp_path);
    }
    result
}

fn create_temp(world_dir: &Path) -> io::Result<(PathBuf, File)> {
    loop {
        let serial = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(".kingdoms_alive-{}-{serial}.tmp", std::process::id());
        let path = world_dir.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}
