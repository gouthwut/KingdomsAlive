use kingdoms_alive_core::{
    lumberjack::{JOURNAL, Journal, Phase, REQUEST, Request},
    settlement::{Settlement, Territory},
};
use std::{
    fs,
    num::{NonZeroU16, NonZeroU64, NonZeroU128},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
struct Dir(PathBuf);
impl Dir {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "ka-wood-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn town() -> Settlement {
    Settlement {
        id: NonZeroU64::MIN,
        name: "Test".into(),
        territory: Territory {
            dimension: "minecraft:overworld".into(),
            center: [0, 70, 0],
            radius: NonZeroU16::new(16).unwrap(),
        },
        residents: [NonZeroU128::MIN].into(),
        stockpile: Default::default(),
        housing_capacity: 1,
    }
}
fn req() -> Request {
    Request {
        job: "test-1".into(),
        resident: NonZeroU128::MIN,
        min: [0, 70, 0],
        max: [1, 70, 0],
    }
}
#[test]
fn ordinary_world_and_revocation_do_not_create_jobs_or_items() {
    let d = Dir::new();
    assert!(Request::open(&d.0, &town()).unwrap().is_none());
    let r = req();
    fs::write(d.0.join(REQUEST), serde_json::to_vec(&r).unwrap()).unwrap();
    assert!(r.still_approved(&d.0, &town()));
    fs::remove_file(d.0.join(REQUEST)).unwrap();
    assert!(!r.still_approved(&d.0, &town()));
    assert!(!d.0.join(JOURNAL).exists());
}
#[test]
fn every_worker_ownership_file_blocks_legacy_even_if_partial() {
    let d = Dir::new();
    assert!(!kingdoms_alive_core::lumberjack::blocks_legacy(&d.0));
    for name in [REQUEST, JOURNAL, ".kingdoms_alive_lumberjack.tmp"] {
        fs::write(d.0.join(name), b"{").unwrap();
        assert!(kingdoms_alive_core::lumberjack::blocks_legacy(&d.0));
        fs::remove_file(d.0.join(name)).unwrap();
    }
}
#[test]
fn invalid_bounds_resident_and_changed_request_are_refused() {
    let d = Dir::new();
    for r in [
        Request {
            max: [i32::MAX, 70, 0],
            ..req()
        },
        Request {
            resident: NonZeroU128::new(2).unwrap(),
            ..req()
        },
        Request {
            min: [2, 70, 0],
            ..req()
        },
        Request {
            max: [0, 70, 17],
            ..req()
        },
    ] {
        fs::write(d.0.join(REQUEST), serde_json::to_vec(&r).unwrap()).unwrap();
        assert!(Request::open(&d.0, &town()).is_err());
    }
    let mut j = Journal::open(&d.0, req()).unwrap();
    j.begin(&d.0, [0, 70, 0]).unwrap();
    assert!(
        Journal::open(
            &d.0,
            Request {
                job: "new".into(),
                ..req()
            }
        )
        .is_err()
    );
}
#[test]
fn restart_each_phase_preserves_receipt_and_blocks_repeat() {
    let d = Dir::new();
    let mut j = Journal::open(&d.0, req()).unwrap();
    j.begin(&d.0, [0, 70, 0]).unwrap();
    assert!(j.begin(&d.0, [1, 70, 0]).is_err());
    assert!(j.transition(&d.0, Phase::Delivered).is_err());
    for phase in [Phase::Carrying, Phase::Depositing, Phase::Delivered] {
        j.transition(&d.0, phase).unwrap();
        j = Journal::open(&d.0, req()).unwrap();
        assert_eq!(j.operations.last().unwrap().phase, phase);
    }
    assert!(j.pending().is_none());
    assert!(j.begin(&d.0, [0, 70, 0]).is_err());
    j.begin(&d.0, [1, 70, 0]).unwrap();
    assert_eq!(j.operations.len(), 2);
}
#[test]
fn partial_or_invalid_journal_is_never_repaired_or_overwritten() {
    let d = Dir::new();
    fs::write(d.0.join(JOURNAL), b"{").unwrap();
    assert!(Journal::open(&d.0, req()).is_err());
    assert_eq!(fs::read(d.0.join(JOURNAL)).unwrap(), b"{");
    fs::remove_file(d.0.join(JOURNAL)).unwrap();
    let mut j = Journal::open(&d.0, req()).unwrap();
    fs::write(d.0.join(".kingdoms_alive_lumberjack.tmp"), b"partial").unwrap();
    assert!(j.begin(&d.0, [0, 70, 0]).is_err());
    assert!(j.operations.is_empty());
    assert!(!d.0.join(JOURNAL).exists());
}
