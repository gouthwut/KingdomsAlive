//! File-boundary regressions, not Horizon crash-consistency tests.
use kingdoms_alive_core::{
    lumberjack::{JOURNAL, Journal, Phase, Request, blocks_legacy},
    recovery::{Observation, RecoveryDecision, Snapshot, reconcile},
};
use std::{
    fs,
    num::NonZeroU128,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "ka-recovery-files-{}-{}",
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
        job: "file-test".into(),
        resident: NonZeroU128::MIN,
        min: [0, 70, 0],
        max: [1, 70, 0],
    }
}
fn unavailable() -> Snapshot {
    Snapshot {
        block: Observation::Unavailable,
        cargo: Observation::Unavailable,
        warehouse: Observation::Unavailable,
    }
}
#[test]
fn every_legacy_phase_reopens_without_replay_or_file_changes() {
    for phase in [
        Phase::Mining,
        Phase::Carrying,
        Phase::Depositing,
        Phase::Delivered,
        Phase::Held,
    ] {
        let dir = Dir::new();
        let mut journal = Journal::open(&dir.0, request()).unwrap();
        journal.begin(&dir.0, [0, 70, 0]).unwrap();
        journal.record_actual_loot(&dir.0, "[]".into()).unwrap();
        match phase {
            Phase::Mining => {}
            Phase::Held => journal.transition(&dir.0, phase).unwrap(),
            _ => {
                journal.transition(&dir.0, Phase::Carrying).unwrap();
                if phase != Phase::Carrying {
                    journal.transition(&dir.0, Phase::Depositing).unwrap();
                }
                if phase == Phase::Delivered {
                    journal.transition(&dir.0, Phase::Delivered).unwrap();
                }
            }
        }
        let original = fs::read(dir.0.join(JOURNAL)).unwrap();
        for _ in 0..2 {
            let mut reopened = Journal::open(&dir.0, request()).unwrap();
            let operation = reopened
                .recovery_operation(&dir.0, &reopened.operations[0])
                .unwrap();
            assert_eq!(
                reconcile(&operation, &unavailable()),
                if phase == Phase::Delivered {
                    RecoveryDecision::AlreadyConfirmed
                } else {
                    RecoveryDecision::Hold
                }
            );
            assert!(reopened.begin(&dir.0, [0, 70, 0]).is_err());
            assert_eq!(fs::read(dir.0.join(JOURNAL)).unwrap(), original);
        }
    }
}
#[test]
fn failed_phase_write_preserves_memory_disk_and_partial_evidence() {
    let dir = Dir::new();
    let mut journal = Journal::open(&dir.0, request()).unwrap();
    journal.begin(&dir.0, [0, 70, 0]).unwrap();
    let original = fs::read(dir.0.join(JOURNAL)).unwrap();
    let tmp = dir.0.join(".kingdoms_alive_lumberjack.tmp");
    fs::write(&tmp, b"partial evidence").unwrap();
    assert!(journal.transition(&dir.0, Phase::Carrying).is_err());
    assert_eq!(journal.pending().unwrap().phase, Phase::Mining);
    assert_eq!(fs::read(dir.0.join(JOURNAL)).unwrap(), original);
    assert_eq!(fs::read(tmp).unwrap(), b"partial evidence");
    assert!(Journal::open(&dir.0, request()).is_err());
    assert!(blocks_legacy(&dir.0));
}
#[test]
fn foreign_and_orphan_receipts_cannot_be_claimed_or_overwritten() {
    let dir = Dir::new();
    let receipt = dir.0.join("kingdoms_alive_lumberjack_loot_0_70_0.json");
    fs::write(&receipt, b"orphan").unwrap();
    let mut journal = Journal::open(&dir.0, request()).unwrap();
    assert!(journal.begin(&dir.0, [0, 70, 0]).is_err());
    assert!(journal.operations.is_empty());
    assert!(blocks_legacy(&dir.0));
    journal.begin(&dir.0, [1, 70, 0]).unwrap();
    let foreign =
        serde_json::json!({"schema":1,"request":request(),"source":[0,70,0],"actual_loot":"[]"});
    let path = dir.0.join("kingdoms_alive_lumberjack_loot_1_70_0.json");
    let bytes = serde_json::to_vec(&foreign).unwrap();
    fs::write(&path, &bytes).unwrap();
    assert!(
        journal
            .recovery_operation(&dir.0, journal.pending().unwrap())
            .is_err()
    );
    assert!(journal.record_actual_loot(&dir.0, "[]".into()).is_err());
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert_eq!(fs::read(receipt).unwrap(), b"orphan");
}
