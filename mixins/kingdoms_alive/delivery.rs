use std::io;
use std::path::{Path, PathBuf};

use horizon_client::items::ItemStack;
use horizon_client::mod_api::EquipmentSlot;
use horizon_core::geometry::Direction;
use horizon_world::nbt::Tag;
use kingdoms_alive_core::delivery::{CLAIM, Request, claim};
use kingdoms_alive_core::settlement::Settlement;

const ITEM: &str = "minecraft:oak_log";

pub(super) struct Delivery {
    request: Request,
    claim: PathBuf,
    started: Option<u64>,
    finished: bool,
}

impl Delivery {
    pub(super) fn open(dir: &Path, settlement: &Settlement) -> io::Result<Option<Self>> {
        let Some(request) = Request::open(dir, settlement)? else {
            return Ok(None);
        };
        let claim = dir.join(CLAIM);
        if claim.try_exists()? {
            log::info!("[kingdoms_alive] M2 claim exists, delivery held for reconciliation");
            return Ok(None);
        }
        Ok(Some(Self {
            request,
            claim,
            started: None,
            finished: false,
        }))
    }

    pub(super) fn tick(&mut self, game: &mut super::Game<'_>, settlement: &Settlement, tick: u64) {
        if self.finished || tick % 5 != 0 {
            return;
        }
        let request = &self.request;
        let territory = &settlement.territory;
        let Some(dir) = self.claim.parent() else {
            self.finished = true;
            return;
        };
        if !request.still_approved(dir, settlement) {
            self.finished = true;
            if let Some((dimension, id, _)) = game.native_entity_by_uuid(request.resident.get()) {
                game.native_mob_stop(&dimension, id);
            }
            log::warn!("[kingdoms_alive] M2 approval revoked, cargo unchanged");
            return;
        }
        let dimension = &territory.dimension;
        // Block container helpers act on the current level only.
        if game.native_dimension() != dimension {
            return;
        }
        let Some(level) = game.native_level(dimension) else {
            return;
        };
        if !game.native_level_is_loaded(level, request.chest)
            || !game.native_level_is_loaded(level, request.approach)
        {
            return;
        }
        if game.native_level_block_state(level, request.chest).as_deref() != Some(&request.chest_state)
            || game.native_level_block(level, request.chest).as_deref() != Some("minecraft:chest")
            || game.native_level_block_property(level, request.chest, "type").as_deref() != Some("single")
            || !game.block_entity_nbt(request.chest).is_some_and(|tag| {
                matches!(tag.get("CustomName"), Some(Tag::String(name)) if name == &request.chest_name.as_str().into())
                    && tag.get("LootTable").is_none()
            })
        {
            self.finished = true;
            log::warn!("[kingdoms_alive] M2 approved chest unavailable");
            return;
        }
        let Some((found_dimension, id, kind)) = game.native_entity_by_uuid(request.resident.get())
        else {
            return;
        };
        if found_dimension != *dimension || kind != "minecraft:villager" {
            return;
        }
        let Some(entity) = game.native_entity(id) else {
            return;
        };
        let Some(pos) = game.native_entity_position(entity) else {
            return;
        };
        let block = pos.map(|axis| axis.floor() as i32);
        if !territory.contains(dimension, block[0], block[2])
            || !game.native_level_is_loaded(level, block)
        {
            return;
        }
        let Some(living) = game.native_living(entity) else {
            return;
        };
        // A villager's trading behavior uses and clears the main hand.
        let hand = EquipmentSlot::OffHand;
        let expected = ItemStack::new(ITEM, 1);
        let Some(stack) = game
            .native_living_equipment(living, hand)
            .filter(|stack| *stack == expected)
        else {
            return;
        };
        let [x, y, z] = request.approach.map(f64::from);
        let target = [x + 0.5, y, z + 0.5];
        let distance_sq: f64 = pos
            .into_iter()
            .zip(target)
            .map(|(a, b)| (a - b).powi(2))
            .sum();
        if distance_sq > 1.0 {
            if self
                .started
                .is_some_and(|start| tick.saturating_sub(start) >= 400)
            {
                self.finished = true;
                game.native_mob_stop(dimension, id);
                log::warn!("[kingdoms_alive] M2 resident did not reach the chest");
            } else {
                let found = game.native_mob_navigate(dimension, id, target, 0, 1.0);
                if self.started.is_none() {
                    self.started = Some(tick);
                    log::info!(
                        "[kingdoms_alive] M2 navigation resident={} from={pos:?} to={target:?} path={found}",
                        request.resident
                    );
                }
            }
            return;
        }

        let Some(slots) = game.native_container_slots(dimension, request.chest, Direction::Up)
        else {
            return;
        };
        let Some(max) = game.native_max_stack_size(ITEM) else {
            return;
        };
        if !slots.iter().any(|slot| {
            slot.stack.as_ref().is_none_or(|at| {
                at.count < max
                    && ItemStack {
                        count: 1,
                        ..at.clone()
                    } == expected
            })
        }) {
            return;
        }
        let before = slots
            .iter()
            .filter_map(|slot| slot.stack.as_ref())
            .filter(|stack| &*stack.item == ITEM)
            .map(|stack| u64::from(stack.count))
            .sum::<u64>();
        if !game.native_mob_stop(dimension, id) {
            return;
        }
        self.finished = true;
        if let Err(error) = claim(&self.claim) {
            log::error!("[kingdoms_alive] M2 claim refused: {error}");
            return;
        }
        if !game.native_living_set_equipment(living, hand, None)
            || game.native_living_equipment(living, hand).is_some()
        {
            log::error!("[kingdoms_alive] M2 hand removal unconfirmed");
            return;
        }
        // shortcut: synchronous entity save for the one-shot pilot, replace with a bounded engine transfer before repeatable jobs.
        if let Err(error) = game.save_entities(true) {
            game.native_living_set_equipment(living, hand, Some(stack));
            log::error!("[kingdoms_alive] M2 hand save failed: {error}");
            return;
        }
        let remainder = game.native_insert_item(request.chest, Direction::Up, stack);
        let after = chest_log_count(game, dimension, request.chest);
        if remainder.is_none() && after == Some(before + 1) {
            log::info!(
                "[kingdoms_alive] M2 delivered resident={} at={pos:?} chest={:?} logs={before}->{}",
                request.resident,
                request.chest,
                before + 1
            );
        } else {
            log::error!("[kingdoms_alive] M2 chest write unconfirmed, reconcile before retry");
        }
    }
}

fn chest_log_count(game: &super::Game<'_>, dimension: &str, chest: [i32; 3]) -> Option<u64> {
    let slots = game.native_container_slots(dimension, chest, Direction::Up)?;
    Some(
        slots
            .into_iter()
            .filter_map(|slot| slot.stack)
            .filter(|stack| &*stack.item == ITEM)
            .map(|stack| u64::from(stack.count))
            .sum(),
    )
}
