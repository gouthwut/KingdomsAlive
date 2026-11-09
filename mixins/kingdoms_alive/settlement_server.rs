use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io;
use std::num::{NonZeroU64, NonZeroU128};
use std::path::{Path, PathBuf};

use crate::mod_api::{NativeLifecycleContext, NativeWorldEpoch};
use kingdoms_alive_core::persistence;
use kingdoms_alive_core::settlement::{Need, Settlement, Territory};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    name: String,
    territory: Territory,
    housing_capacity: u32,
}

struct Pending {
    registration: Registration,
    residents: BTreeSet<NonZeroU128>,
    dir: PathBuf,
}

enum World {
    Idle,
    Pending(Pending),
    Active {
        settlement: Settlement,
        last_need: Option<Option<Need>>,
    },
}

thread_local! {
    static WORLD: RefCell<Option<(NativeWorldEpoch, World)>> = const { RefCell::new(None) };
}

fn activate(settlement: Settlement) -> World {
    World::Active {
        settlement,
        last_need: None,
    }
}

fn open(dir: &Path) -> io::Result<World> {
    if let Some(settlement) = persistence::load(dir)? {
        log::info!("[kingdoms_alive] M1 loaded id={}", settlement.id);
        return Ok(activate(settlement));
    }

    let bytes = match std::fs::read(dir.join("kingdoms_alive_register.json")) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(World::Idle),
        Err(error) => return Err(error),
    };
    let registration: Registration = serde_json::from_slice(&bytes)?;
    let radius = i32::from(registration.territory.radius.get());
    let [x, _, z] = registration.territory.center;
    if registration.name.trim().is_empty()
        || radius > 128
        || [x, z]
            .iter()
            .any(|&axis| axis.checked_sub(radius).is_none() || axis.checked_add(radius).is_none())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "registration requires a name and valid bounds with radius 1–128",
        ));
    }
    log::info!("[kingdoms_alive] M1 waiting for villagers in approved territory");
    Ok(World::Pending(Pending {
        registration,
        residents: BTreeSet::new(),
        dir: dir.to_path_buf(),
    }))
}

pub(super) fn observing() -> bool {
    WORLD.with_borrow(|state| matches!(state, Some((_, World::Pending(_)))))
}

pub(super) fn observe(game: &mut super::Game<'_>, dimension: &str, entity: u32) {
    WORLD.with_borrow_mut(|state| {
        let Some((_, World::Pending(pending))) = state else {
            return;
        };
        let territory = &pending.registration.territory;
        if territory.dimension != dimension {
            return;
        }
        let Some(entity) = game.native_entity(entity) else {
            return;
        };
        let Some([x, _, z]) = game.native_entity_block_position(entity) else {
            return;
        };
        if territory.contains(dimension, x, z)
            && let Some(uuid) = game.native_entity_uuid(entity).and_then(NonZeroU128::new)
        {
            pending.residents.insert(uuid);
        }
    });
}

fn territory_loaded(game: &super::Game<'_>, territory: &Territory) -> bool {
    let Some(level) = game.native_level(&territory.dimension) else {
        return false;
    };
    let [x, y, z] = territory.center;
    let radius = i32::from(territory.radius.get());
    let chunk_x = ((x - radius) >> 4)..=((x + radius) >> 4);
    let chunk_z = ((z - radius) >> 4)..=((z + radius) >> 4);
    chunk_x.into_iter().all(|cx| {
        chunk_z
            .clone()
            .all(|cz| game.native_level_is_loaded(level, [cx << 4, y, cz << 4]))
    })
}

pub(super) fn tick(game: &super::Game<'_>, context: &NativeLifecycleContext) {
    let tick = context.tick.get();
    WORLD.with_borrow_mut(|state| {
        if state.as_ref().is_some_and(|(epoch, _)| *epoch != context.epoch) {
            *state = None;
        }
        if let Some((_, World::Pending(pending))) = state
            && tick % 20 != 0
        {
            pending.residents.clear();
        }
        if tick % 20 != 0 {
            return;
        }

        if state.is_none() {
            let Some(dir) = game.native_world_dir() else {
                return;
            };
            let world = open(&dir).unwrap_or_else(|error| {
                log::error!("[kingdoms_alive] M1 open refused: {error}");
                World::Idle
            });
            *state = Some((context.epoch, world));
        }

        let Some((_, world)) = state else {
            return;
        };
        if let World::Pending(pending) = world {
            let residents = std::mem::take(&mut pending.residents);
            if residents.is_empty() || !territory_loaded(game, &pending.registration.territory) {
                return;
            }
            let settlement = Settlement {
                id: NonZeroU64::MIN,
                name: pending.registration.name.clone(),
                territory: pending.registration.territory.clone(),
                residents,
                stockpile: Default::default(),
                housing_capacity: pending.registration.housing_capacity,
            };
            match persistence::save(&pending.dir, &settlement) {
                Ok(()) => {
                    log::info!("[kingdoms_alive] M1 registered id={}", settlement.id);
                    *world = activate(settlement);
                }
                Err(error) => {
                    log::error!("[kingdoms_alive] M1 registration refused: {error}");
                    *world = World::Idle;
                }
            }
        }
        if let World::Active { settlement, last_need } = world {
            let need = settlement.next_need(16, 16);
            if *last_need != Some(need) {
                log::info!(
                    "[kingdoms_alive] M1 tick={tick} id={} residents={} stockpile={:?} need={need:?}",
                    settlement.id,
                    settlement.residents.len(),
                    settlement.stockpile,
                );
                *last_need = Some(need);
            }
        }
    });
}

pub(super) fn stop() {
    WORLD.with_borrow_mut(|state| *state = None);
}
