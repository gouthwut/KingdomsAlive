//! Bounded, explicitly approved lumberjack jobs and a conservative operation journal.
//! An interrupted mutation is held, never replayed or compensated from cached counts.
use crate::settlement::Settlement;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write},
    num::NonZeroU128,
    path::Path,
};

pub const REQUEST: &str = "kingdoms_alive_lumberjack.json";
pub const JOURNAL: &str = "kingdoms_alive_lumberjack_journal.json";

/// Conservative controller exclusion, including unreadable or partial ownership files.
/// Completed journals are retained too: legacy delivery must never take worker cargo.
pub fn blocks_legacy(dir: &Path) -> bool {
    [REQUEST, JOURNAL, ".kingdoms_alive_lumberjack.tmp"]
        .iter()
        .any(|name| dir.join(name).try_exists().unwrap_or(true))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub job: String,
    pub resident: NonZeroU128,
    pub min: [i32; 3],
    pub max: [i32; 3],
}
impl Request {
    pub fn open(dir: &Path, town: &Settlement) -> io::Result<Option<Self>> {
        let bytes = match fs::read(dir.join(REQUEST)) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let r: Self = serde_json::from_slice(&bytes)?;
        if r.job.trim().is_empty()
            || r.job.len() > 64
            || !town.residents.contains(&r.resident)
            || !(0..3).all(|i| {
                r.min[i] <= r.max[i] && r.min[i].abs_diff(r.max[i]) <= if i == 1 { 3 } else { 7 }
            })
            || ![r.min, r.max].iter().all(|p| {
                town.territory
                    .contains(&town.territory.dimension, p[0], p[2])
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid lumberjack approval",
            ));
        }
        Ok(Some(r))
    }
    pub fn still_approved(&self, dir: &Path, town: &Settlement) -> bool {
        matches!(Self::open(dir,town),Ok(Some(r)) if r == *self)
    }
    pub fn positions(&self) -> impl Iterator<Item = [i32; 3]> + '_ {
        (self.min[0]..=self.max[0]).flat_map(move |x| {
            (self.min[1]..=self.max[1])
                .flat_map(move |y| (self.min[2]..=self.max[2]).map(move |z| [x, y, z]))
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum Phase {
    Mining,
    Carrying,
    Depositing,
    Delivered,
    Held,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub source: [i32; 3],
    pub phase: Phase,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Journal {
    pub schema: u32,
    pub request: Request,
    pub operations: Vec<Operation>,
}
impl Journal {
    pub fn open(dir: &Path, request: Request) -> io::Result<Self> {
        let bytes = match fs::read(dir.join(JOURNAL)) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(Self {
                    schema: 1,
                    request,
                    operations: Vec::new(),
                });
            }
            Err(e) => return Err(e),
        };
        let j: Self = serde_json::from_slice(&bytes)?;
        let mut seen = std::collections::BTreeSet::new();
        if j.schema != 1
            || j.request != request
            || j.operations.len() > 256
            || j.operations.iter().any(|op| {
                !seen.insert(op.source)
                    || !(0..3)
                        .all(|i| op.source[i] >= request.min[i] && op.source[i] <= request.max[i])
            })
            || j.operations
                .iter()
                .rev()
                .skip(1)
                .any(|op| op.phase != Phase::Delivered)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid or changed lumberjack journal",
            ));
        }
        Ok(j)
    }
    pub fn used(&self, p: [i32; 3]) -> bool {
        self.operations.iter().any(|op| op.source == p)
    }
    pub fn pending(&self) -> Option<&Operation> {
        self.operations
            .iter()
            .find(|op| op.phase != Phase::Delivered)
    }
    pub fn begin(&mut self, dir: &Path, source: [i32; 3]) -> io::Result<()> {
        if self.pending().is_some()
            || self.used(source)
            || !self.request.positions().any(|p| p == source)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "operation is not new",
            ));
        }
        let mut next = self.clone();
        next.operations.push(Operation {
            source,
            phase: Phase::Mining,
        });
        next.save(dir)?;
        *self = next;
        Ok(())
    }
    pub fn transition(&mut self, dir: &Path, phase: Phase) -> io::Result<()> {
        let current = self
            .pending()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no pending operation"))?
            .phase;
        if !matches!(
            (current, phase),
            (Phase::Mining, Phase::Carrying)
                | (Phase::Carrying, Phase::Depositing)
                | (Phase::Depositing, Phase::Delivered)
        ) && phase != Phase::Held
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "unsafe operation transition",
            ));
        }
        let mut next = self.clone();
        next.operations.last_mut().expect("pending operation").phase = phase;
        next.save(dir)?;
        *self = next;
        Ok(())
    }
    fn save(&self, dir: &Path) -> io::Result<()> {
        // Fixed temporary name is exclusive; an uncertain previous write blocks further mutations.
        let path = dir.join(".kingdoms_alive_lumberjack.tmp");
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        f.write_all(&serde_json::to_vec_pretty(self)?)?;
        f.sync_all()?;
        drop(f);
        fs::rename(path, dir.join(JOURNAL))
    }
}
