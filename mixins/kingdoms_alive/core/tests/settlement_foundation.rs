use std::fs;
use std::io::ErrorKind;
use std::num::{NonZeroU16, NonZeroU64, NonZeroU128};
use std::path::PathBuf;

use kingdoms_alive_core::persistence::{load, save};
use kingdoms_alive_core::settlement::{Need, Settlement, Stockpile, Territory};

fn settlement() -> Settlement {
    Settlement {
        id: NonZeroU64::MIN,
        name: "Берёзовый берег".into(),
        territory: Territory {
            dimension: "minecraft:overworld".into(),
            center: [10, 70, -20],
            radius: NonZeroU16::new(16).unwrap(),
        },
        residents: [NonZeroU128::MIN, NonZeroU128::MAX].into(),
        stockpile: Stockpile::default(),
        housing_capacity: 2,
    }
}

#[test]
fn need_changes_with_population_housing_and_stock() {
    let mut town = settlement();
    assert_eq!(town.next_need(8, 8), Some(Need::Food));
    town.stockpile.food = 2;
    assert_eq!(town.next_need(8, 8), Some(Need::Housing));
    town.housing_capacity = 3;
    assert_eq!(town.next_need(8, 8), Some(Need::Wood));
    town.stockpile.wood = 8;
    assert_eq!(town.next_need(8, 8), Some(Need::Stone));
    town.stockpile.stone = 8;
    assert_eq!(town.next_need(8, 8), None);
    town.residents.clear();
    assert_eq!(town.next_need(9, 9), None);
}

#[test]
fn territory_respects_dimension_and_boundary_without_coordinate_overflow() {
    let area = settlement().territory;
    assert!(area.contains("minecraft:overworld", 26, -36));
    assert!(!area.contains("minecraft:overworld", 27, -36));
    assert!(!area.contains("minecraft:the_nether", 10, -20));
    assert!(!area.contains("minecraft:overworld", i32::MIN, i32::MAX));
}

#[test]
fn snapshot_survives_reload_and_replacement() {
    let world = TestWorld::new("roundtrip");
    let mut town = settlement();
    assert_eq!(load(&world.0).unwrap(), None);
    save(&world.0, &town).unwrap();
    assert_eq!(load(&world.0).unwrap(), Some(town.clone()));

    town.stockpile.wood = 24;
    save(&world.0, &town).unwrap();
    assert_eq!(load(&world.0).unwrap(), Some(town));
    assert_eq!(fs::read_dir(&world.0).unwrap().count(), 1);
}

#[test]
fn unreadable_or_future_snapshot_is_preserved() {
    let world = TestWorld::new("invalid");
    let path = world.0.join("kingdoms_alive.json");
    for original in [b"{".as_slice(), br#"{"schema":2,"other":"future"}"#] {
        fs::write(&path, original).unwrap();
        assert_eq!(load(&world.0).unwrap_err().kind(), ErrorKind::InvalidData);
        assert_eq!(
            save(&world.0, &settlement()).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        assert_eq!(fs::read(&path).unwrap(), original);
    }
}

struct TestWorld(PathBuf);

impl TestWorld {
    fn new(label: &str) -> Self {
        for attempt in 0..100 {
            let path = std::env::temp_dir().join(format!(
                "kingdoms-alive-test-{label}-{}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create test world: {error}"),
            }
        }
        panic!("no free test world path");
    }
}

impl Drop for TestWorld {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0.join("kingdoms_alive.json"));
        let _ = fs::remove_dir(&self.0);
    }
}
