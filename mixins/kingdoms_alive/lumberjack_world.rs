// Thin physical-world primitives. No item is manufactured from a journal or stock count.
use horizon_client::{items::ItemStack, mod_api::EquipmentSlot};
use horizon_core::geometry::Direction;
use horizon_world::nbt::Tag;
use kingdoms_alive_core::settlement::Settlement;
use kingdoms_alive_core::warehouse::{Chest, Registry};
use std::path::Path;
pub(super) const ITEM: &str = "minecraft:oak_log";

pub(super) fn chest_valid(game: &super::Game<'_>, town: &Settlement, chest: &Chest) -> bool {
    let Some(level) = game.native_level(&town.territory.dimension) else {
        return false;
    };
    game.native_dimension()==town.territory.dimension
        && game.native_level_is_loaded(level,chest.pos)
        && game.native_level_block_state(level,chest.pos).as_deref()==Some(&chest.state)
        && game.native_level_block(level,chest.pos).as_deref()==Some("minecraft:chest")
        && game.native_level_block_property(level,chest.pos,"type").as_deref()==Some("single")
        && game.block_entity_nbt(chest.pos).is_some_and(|tag|matches!(tag.get("CustomName"),Some(Tag::String(name)) if name == &chest.name.as_str().into()) && tag.get("LootTable").is_none())
}
pub(super) fn destination(
    game: &super::Game<'_>,
    dir: &Path,
    town: &Settlement,
) -> Option<(Chest, [i32; 3])> {
    let registry = Registry::open(dir, town).ok()??;
    let level = game.native_level(&town.territory.dimension)?;
    for chest in registry.chests {
        if !chest_valid(game, town, &chest) {
            continue;
        }
        let slots = game.native_container_slots(&registry.dimension, chest.pos, Direction::Up)?;
        if !slots.iter().any(|s| {
            s.stack.as_ref().is_none_or(|s| {
                s.count < 64
                    && ItemStack {
                        count: 1,
                        ..s.clone()
                    } == ItemStack::new(ITEM, 1)
            })
        }) {
            continue;
        }
        for [dx, dz] in [[-1, 0], [0, -1], [1, 0], [0, 1]] {
            let [x, y, z] = chest.pos;
            let (Some(x), Some(z), Some(above), Some(below)) = (
                x.checked_add(dx),
                z.checked_add(dz),
                y.checked_add(1),
                y.checked_sub(1),
            ) else {
                continue;
            };
            let at = [x, y, z];
            if town.territory.contains(&registry.dimension, x, z)
                && game.native_level_is_loaded(level, at)
                && game.native_level_block(level, at).as_deref() == Some("minecraft:air")
                && game.native_level_block(level, [x, above, z]).as_deref() == Some("minecraft:air")
                && matches!(
                    game.native_level_block(level, [x, below, z]).as_deref(),
                    Some(
                        "minecraft:stone"
                            | "minecraft:dirt"
                            | "minecraft:grass_block"
                            | "minecraft:cobblestone"
                    )
                )
            {
                return Some((chest, at));
            }
        }
    }
    None
}
pub(super) fn hand(
    game: &mut super::Game<'_>,
    resident: u128,
) -> Option<(u32, Option<ItemStack>, [f64; 3])> {
    let (dim, id, kind) = game.native_entity_by_uuid(resident)?;
    if dim != game.native_dimension() || kind != "minecraft:villager" {
        return None;
    }
    let entity = game.native_entity(id)?;
    let pos = game.native_entity_position(entity)?;
    let living = game.native_living(entity)?;
    Some((
        id,
        game.native_living_equipment(living, EquipmentSlot::OffHand),
        pos,
    ))
}
pub(super) fn set_hand(game: &mut super::Game<'_>, id: u32, stack: Option<ItemStack>) -> bool {
    let Some(e) = game.native_entity(id) else {
        return false;
    };
    let Some(l) = game.native_living(e) else {
        return false;
    };
    game.native_living_set_equipment(l, EquipmentSlot::OffHand, stack.clone())
        && game.native_living_equipment(l, EquipmentSlot::OffHand) == stack
}
pub(super) fn count(game: &super::Game<'_>, pos: [i32; 3]) -> Option<u64> {
    Some(
        game.native_container_slots(game.native_dimension(), pos, Direction::Up)?
            .iter()
            .filter_map(|s| s.stack.as_ref())
            .filter(|s| &*s.item == ITEM)
            .map(|s| u64::from(s.count))
            .sum(),
    )
}
