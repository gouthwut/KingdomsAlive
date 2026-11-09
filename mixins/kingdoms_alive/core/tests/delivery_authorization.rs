use std::fs;
use std::num::{NonZeroU16, NonZeroU64, NonZeroU128};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use kingdoms_alive_core::delivery::{CLAIM, REQUEST, Request, claim};
use kingdoms_alive_core::settlement::{Settlement, Territory};

struct World(PathBuf);
impl World {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ka-delivery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
    fn write(&self, value: &serde_json::Value) {
        fs::write(self.0.join(REQUEST), serde_json::to_vec(value).unwrap()).unwrap();
    }
}
impl Drop for World {
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
fn request() -> serde_json::Value {
    serde_json::json!({
        "resident": 1,
        "chest": [0, 70, 0],
        "approach": [1, 70, 0],
        "chest_state": "minecraft:chest[facing=north,type=single,waterlogged=false]",
        "chest_name": "\"Test warehouse\""
    })
}

#[test]
fn ordinary_world_has_no_delivery_or_claim() {
    let w = World::new();
    assert_eq!(Request::open(&w.0, &town()).unwrap(), None);
    assert!(!w.0.join(CLAIM).exists());
}

#[test]
fn approval_can_be_revoked_without_restarting_world() {
    let w = World::new();
    let original = request();
    w.write(&original);
    let active = Request::open(&w.0, &town()).unwrap().unwrap();
    assert!(active.still_approved(&w.0, &town()));
    let mut changed = original.clone();
    changed["chest_name"] = serde_json::json!("replacement");
    w.write(&changed);
    assert!(!active.still_approved(&w.0, &town()));
    w.write(&original);
    fs::remove_file(w.0.join(REQUEST)).unwrap();
    assert!(!active.still_approved(&w.0, &town()));
    fs::write(w.0.join(REQUEST), b"{").unwrap();
    assert!(!active.still_approved(&w.0, &town()));
}

#[test]
fn unknown_resident_boundary_and_invalid_approach_are_refused() {
    let w = World::new();
    for (key, value) in [
        ("resident", serde_json::json!(2)),
        ("chest", serde_json::json!([17, 70, 0])),
        ("approach", serde_json::json!([0, 70, 0])),
        ("approach", serde_json::json!([1, 71, 0])),
        ("approach", serde_json::json!([i32::MIN, 70, i32::MAX])),
        ("chest_name", serde_json::json!(" ")),
        ("unexpected", serde_json::json!(true)),
    ] {
        let mut input = request();
        input[key] = value;
        w.write(&input);
        assert!(
            Request::open(&w.0, &town()).is_err(),
            "accepted {key}: {input}"
        );
    }
}

#[test]
fn existing_and_partial_claims_cannot_be_replayed_or_overwritten() {
    let w = World::new();
    let path = w.0.join(CLAIM);
    claim(&path).unwrap();
    let original = fs::read(&path).unwrap();
    assert_eq!(
        claim(&path).unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    fs::write(&path, b"").unwrap();
    assert_eq!(
        claim(&path).unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert!(fs::read(&path).unwrap().is_empty());
}
