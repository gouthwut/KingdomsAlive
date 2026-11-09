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

/// No proven durable ordering exists between the journal, blocks, entities and chest.
/// This is deliberately not a per-world opt-in: approval cannot waive crash safety.
pub const MUTATIONS_ENABLED: bool = false;

/// Immutable sidecar for one result; schema-1 journals remain byte-compatible.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LootReceipt {
    schema: u32,
    request: Request,
    source: [i32; 3],
    actual_loot: String,
}

fn loot_path(dir: &Path, source: [i32; 3]) -> std::path::PathBuf {
    let [x, y, z] = source;
    dir.join(format!("kingdoms_alive_lumberjack_loot_{x}_{y}_{z}.json"))
}

/// Conservative controller exclusion, including unreadable or partial ownership files.
/// Completed journals are retained too: legacy delivery must never take worker cargo.
pub fn blocks_legacy(dir: &Path) -> bool {
    [REQUEST, JOURNAL, ".kingdoms_alive_lumberjack.tmp"]
        .iter()
        .any(|name| dir.join(name).try_exists().unwrap_or(true))
        || fs::read_dir(dir).map_or(true, |mut entries| {
            // A world with too many entries is unknown, not an unbounded tick scan.
            for _ in 0..512 {
                match entries.next() {
                    None => return false,
                    Some(Err(_)) => return true,
                    Some(Ok(entry)) => {
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with("kingdoms_alive_lumberjack_loot_")
                        {
                            return true;
                        }
                    }
                }
            }
            true
        })
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
        if dir.join(".kingdoms_alive_lumberjack.tmp").try_exists()? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unfinished lumberjack journal write; reconciliation required",
            ));
        }
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
    /// Project the old journal into the diagnostic contract. Legacy records contain
    /// no complete endpoint snapshots: never invent them from current item counts.
    pub fn recovery_operation(
        &self,
        dir: &Path,
        op: &Operation,
    ) -> io::Result<crate::recovery::Operation> {
        use crate::recovery::{Observation, Phase as RecoveryPhase, Snapshot};
        let actual_loot = match fs::read(loot_path(dir, op.source)) {
            Ok(bytes) => {
                let receipt: LootReceipt = serde_json::from_slice(&bytes)?;
                if receipt.schema != 1
                    || receipt.request != self.request
                    || receipt.source != op.source
                {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "loot receipt does not match operation",
                    ));
                }
                // The legacy API accepts JSON, but a non-conforming receipt must
                // never be interpreted as a complete engine result.
                let _: serde_json::Value = serde_json::from_str(&receipt.actual_loot)?;
                Some(receipt.actual_loot)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        let unknown = Snapshot {
            block: Observation::Unavailable,
            cargo: Observation::Unavailable,
            warehouse: Observation::Unavailable,
        };
        Ok(crate::recovery::Operation {
            id: serde_json::to_string(&(&self.request, op.source))?,
            phase: match op.phase {
                Phase::Mining => RecoveryPhase::Prepared,
                Phase::Carrying | Phase::Depositing => RecoveryPhase::Executed,
                Phase::Delivered => RecoveryPhase::Confirmed,
                Phase::Held => RecoveryPhase::Uncertain,
            },
            actual_loot,
            before: unknown.clone(),
            after: unknown,
        })
    }
    /// Record the complete serialized result immediately after the engine returns it,
    /// before hand mutation, item spawning, or further save calls. Never overwrite.
    /// This still cannot close the crash window inside the destructive engine call.
    pub fn record_actual_loot(&self, dir: &Path, actual_loot: String) -> io::Result<()> {
        let op = self
            .pending()
            .filter(|op| op.phase == Phase::Mining)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no harvest intent"))?;
        let _: serde_json::Value = serde_json::from_str(&actual_loot)?;
        let receipt = LootReceipt {
            schema: 1,
            request: self.request.clone(),
            source: op.source,
            actual_loot,
        };
        let bytes = serde_json::to_vec_pretty(&receipt)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(loot_path(dir, op.source))?;
        file.write_all(&bytes)?;
        file.sync_all()
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
            || loot_path(dir, source).try_exists()?
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

#[cfg(test)]
mod recovery_adapter_tests {
    use super::*;
    use crate::recovery::{Observation, RecoveryDecision, Snapshot, reconcile};
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Dir(std::path::PathBuf);
    impl Dir {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "ka-loot-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Dir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn request() -> Request {
        Request {
            job: "receipt-test".into(),
            resident: NonZeroU128::MIN,
            min: [0, 70, 0],
            max: [0, 70, 0],
        }
    }
    fn observed() -> Snapshot {
        Snapshot {
            block: Observation::Available("air".into()),
            cargo: Observation::Available("empty".into()),
            warehouse: Observation::Available("empty".into()),
        }
    }
    #[test]
    fn receipt_survives_reopen_without_authorizing_replay_or_migrating_journal() {
        let dir = Dir::new();
        let mut journal = Journal::open(&dir.0, request()).unwrap();
        journal.begin(&dir.0, [0, 70, 0]).unwrap();
        let original = fs::read(dir.0.join(JOURNAL)).unwrap();
        let loot =
            r#"[{"item":"minecraft:oak_log","count":1,"components":{"test:data":{"value":7}}}]"#;
        journal.record_actual_loot(&dir.0, loot.into()).unwrap();
        assert_eq!(fs::read(dir.0.join(JOURNAL)).unwrap(), original);
        let reopened = Journal::open(&dir.0, request()).unwrap();
        let operation = reopened
            .recovery_operation(&dir.0, reopened.pending().unwrap())
            .unwrap();
        assert_eq!(operation.actual_loot.as_deref(), Some(loot));
        assert_eq!(reconcile(&operation, &observed()), RecoveryDecision::Hold);
        assert!(journal.begin(&dir.0, [0, 70, 0]).is_err());
    }
    #[test]
    fn empty_loot_is_known_but_legacy_missing_snapshots_still_hold() {
        let dir = Dir::new();
        let mut journal = Journal::open(&dir.0, request()).unwrap();
        journal.begin(&dir.0, [0, 70, 0]).unwrap();
        journal.record_actual_loot(&dir.0, "[]".into()).unwrap();
        let operation = journal
            .recovery_operation(&dir.0, journal.pending().unwrap())
            .unwrap();
        assert_eq!(operation.actual_loot.as_deref(), Some("[]"));
        assert_eq!(reconcile(&operation, &observed()), RecoveryDecision::Hold);
        let path = loot_path(&dir.0, [0, 70, 0]);
        let original = fs::read(&path).unwrap();
        assert!(journal.record_actual_loot(&dir.0, "[1]".into()).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
    }
    #[test]
    fn missing_or_partial_receipt_never_allows_continuation() {
        let dir = Dir::new();
        let mut journal = Journal::open(&dir.0, request()).unwrap();
        journal.begin(&dir.0, [0, 70, 0]).unwrap();
        let operation = journal
            .recovery_operation(&dir.0, journal.pending().unwrap())
            .unwrap();
        assert!(operation.actual_loot.is_none());
        assert_eq!(reconcile(&operation, &observed()), RecoveryDecision::Hold);
        let path = loot_path(&dir.0, [0, 70, 0]);
        fs::write(&path, b"{").unwrap();
        assert!(
            journal
                .recovery_operation(&dir.0, journal.pending().unwrap())
                .is_err()
        );
        assert!(journal.record_actual_loot(&dir.0, "[]".into()).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"{");
    }
    #[test]
    fn stale_temporary_write_blocks_open_and_orphan_receipt_blocks_legacy() {
        let dir = Dir::new();
        fs::write(dir.0.join(".kingdoms_alive_lumberjack.tmp"), b"{").unwrap();
        assert!(Journal::open(&dir.0, request()).is_err());
        fs::remove_file(dir.0.join(".kingdoms_alive_lumberjack.tmp")).unwrap();
        fs::write(loot_path(&dir.0, [0, 70, 0]), b"{").unwrap();
        assert!(blocks_legacy(&dir.0));
    }
}
