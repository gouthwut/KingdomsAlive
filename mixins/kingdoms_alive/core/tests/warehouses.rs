use kingdoms_alive_core::settlement::{Settlement, Stockpile, Territory};
use kingdoms_alive_core::warehouse::{MAX_CHESTS, Registry, count};
use std::num::{NonZeroU16, NonZeroU64};

fn town() -> Settlement {
    Settlement {
        id: NonZeroU64::MIN,
        name: "Test".into(),
        territory: Territory {
            dimension: "minecraft:overworld".into(),
            center: [0, 70, 0],
            radius: NonZeroU16::new(16).unwrap(),
        },
        residents: Default::default(),
        stockpile: Stockpile {
            food: 99,
            wood: 99,
            stone: 99,
        },
        housing_capacity: 0,
    }
}
fn value() -> serde_json::Value {
    serde_json::json!({"schema":1,"settlement":1,"dimension":"minecraft:overworld","chests":[{"pos":[0,70,0],"state":"minecraft:chest[facing=north,type=single,waterlogged=false]","name":"\"Warehouse\""}]})
}
fn valid(v: serde_json::Value) -> bool {
    serde_json::from_value::<Registry>(v).is_ok_and(|r| r.validate(&town()).is_ok())
}
#[test]
fn approval_checks_owner_dimension_boundary_schema_and_single_chest() {
    assert!(valid(value()));
    for (key, bad) in [
        ("schema", serde_json::json!(2)),
        ("settlement", serde_json::json!(2)),
        ("dimension", serde_json::json!("minecraft:the_nether")),
        ("extra", serde_json::json!(true)),
    ] {
        let mut v = value();
        v[key] = bad;
        assert!(!valid(v), "{key}");
    }
    for (key, bad) in [
        ("pos", serde_json::json!([17, 70, 0])),
        ("name", serde_json::json!(" ")),
        ("state", serde_json::json!("minecraft:chest[type=left]")),
    ] {
        let mut v = value();
        v["chests"][0][key] = bad;
        assert!(!valid(v), "{key}");
    }
}
#[test]
fn duplicate_chests_cannot_double_count_and_scan_is_bounded() {
    let mut v = value();
    let entry = v["chests"][0].clone();
    v["chests"].as_array_mut().unwrap().push(entry);
    assert!(!valid(v));
    let mut v = value();
    let entries = (0..=MAX_CHESTS)
        .map(|i| {
            let mut c = v["chests"][0].clone();
            c["pos"] = serde_json::json!([i, 70, 0]);
            c
        })
        .collect::<Vec<_>>();
    v["chests"] = serde_json::json!(entries);
    assert!(!valid(v));
}
#[test]
fn counts_come_from_observations_not_history_and_can_decrease() {
    let a = count([
        ("minecraft:oak_log", 12),
        ("minecraft:bread", 4),
        ("minecraft:stone", 2),
        ("minecraft:diamond", 64),
    ])
    .unwrap();
    assert_eq!(
        a,
        Stockpile {
            food: 4,
            wood: 12,
            stone: 2
        }
    );
    assert_eq!(count([("minecraft:oak_log", 1)]).unwrap().wood, 1);
    assert_eq!(count([]).unwrap(), Stockpile::default());
}
#[test]
fn overflowing_observation_is_unknown_not_a_saturated_balance() {
    assert_eq!(
        count([("minecraft:oak_log", u32::MAX), ("minecraft:birch_log", 1)]),
        None
    );
}
