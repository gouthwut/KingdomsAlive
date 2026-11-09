use std::path::PathBuf;

use horizon_core::geometry::Direction;
use horizon_world::nbt::Tag;
use kingdoms_alive_core::settlement::{Settlement, Stockpile};
use kingdoms_alive_core::warehouse::{Registry, count};

pub(super) struct Warehouses {
    dir: PathBuf,
    enabled: bool,
    last: Option<Option<Stockpile>>,
}

impl Warehouses {
    pub(super) fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            enabled: false,
            last: None,
        }
    }

    /// None is unknown. Never substitute a stale or partially observed count.
    /// At most eight single chests (216 slots) are scanned per settlement step.
    pub(super) fn observe(
        &mut self,
        game: &super::Game<'_>,
        town: &Settlement,
    ) -> Option<Stockpile> {
        let registry = match Registry::open(&self.dir, town) {
            Ok(Some(registry)) => {
                self.enabled = true;
                Some(registry)
            }
            Ok(None) if !self.enabled => return Some(Stockpile::default()), // No approved warehouse; never use historical M1 counts.
            _ => {
                self.enabled = true;
                None
            }
        };
        let stock = registry.and_then(|registry| scan(game, &registry));
        if self.last != Some(stock) {
            log::info!("[kingdoms_alive] M2 physical stock={stock:?}");
            self.last = Some(stock);
        }
        stock
    }
}

fn scan(game: &super::Game<'_>, registry: &Registry) -> Option<Stockpile> {
    if game.native_dimension() != registry.dimension {
        return None;
    }
    let level = game.native_level(&registry.dimension)?;
    let mut stacks = Vec::new();
    for chest in &registry.chests {
        if !game.native_level_is_loaded(level, chest.pos)
            || game.native_level_block(level, chest.pos).as_deref() != Some("minecraft:chest")
            || game.native_level_block_property(level, chest.pos, "type").as_deref() != Some("single")
            || game.native_level_block_state(level, chest.pos).as_deref() != Some(&chest.state)
            || !game.block_entity_nbt(chest.pos).is_some_and(|tag| {
                matches!(tag.get("CustomName"), Some(Tag::String(name)) if name == &chest.name.as_str().into())
                    && tag.get("LootTable").is_none()
            })
        { return None; }
        let slots = game.native_container_slots(&registry.dimension, chest.pos, Direction::Up)?;
        if slots.len() != 27 {
            return None;
        }
        stacks.extend(slots.into_iter().filter_map(|slot| slot.stack));
    }
    count(stacks.iter().map(|stack| (&*stack.item, stack.count)))
}
