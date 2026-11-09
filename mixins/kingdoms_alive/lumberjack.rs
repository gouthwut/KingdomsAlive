// Experimental oak-only worker. Interrupted destructive phases stop for reconciliation.
use super::kingdoms_alive_lumberjack_world as world;
use horizon_client::items::ItemStack;
use horizon_core::geometry::Direction;
use kingdoms_alive_core::{
    lumberjack::{Journal, Phase, Request},
    settlement::Settlement,
};
use std::path::{Path, PathBuf};

pub(super) struct Lumberjack {
    dir: PathBuf,
    journal: Journal,
    target: Option<[i32; 3]>,
    started: Option<u64>,
    stopped: bool,
}
impl Lumberjack {
    pub(super) fn open(dir: &Path, town: &Settlement) -> std::io::Result<Option<Self>> {
        let Some(request) = Request::open(dir, town)? else {
            return Ok(None);
        };
        // Two controllers must never own one resident or physical cargo.
        if dir
            .join(kingdoms_alive_core::delivery::REQUEST)
            .try_exists()?
            || dir
                .join(kingdoms_alive_core::delivery::CLAIM)
                .try_exists()?
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "remove legacy delivery approval before lumberjack work",
            ));
        }
        let journal = Journal::open(dir, request)?;
        // An untagged hand cannot prove ownership after external mutation during downtime.
        // Even Carrying requires reconciliation; never assume a replacement log is this cargo.
        let stopped = journal.pending().is_some();
        if stopped {
            log::warn!("[kingdoms_alive] M2 lumberjack interrupted operation held; no replay");
        }
        Ok(Some(Self {
            dir: dir.into(),
            journal,
            target: None,
            started: None,
            stopped,
        }))
    }
    fn held(&mut self, reason: &str) {
        self.stopped = true;
        log::warn!("[kingdoms_alive] M2 lumberjack held: {reason}");
    }
    pub(super) fn tick(&mut self, game: &mut super::Game<'_>, town: &Settlement, tick: u64) {
        if self.stopped || tick % 5 != 0 {
            return;
        }
        if self
            .dir
            .join(kingdoms_alive_core::delivery::REQUEST)
            .try_exists()
            .unwrap_or(true)
            || self
                .dir
                .join(kingdoms_alive_core::delivery::CLAIM)
                .try_exists()
                .unwrap_or(true)
            || !self.journal.request.still_approved(&self.dir, town)
        {
            if let Some((id, _, _)) = world::hand(game, self.journal.request.resident.get()) {
                game.native_mob_stop(&town.territory.dimension, id);
            }
            self.held("approval revoked; physical cargo retained");
            return;
        }
        if game.native_dimension() != town.territory.dimension {
            return;
        }
        let Some((id, hand, pos)) = world::hand(game, self.journal.request.resident.get()) else {
            return;
        };
        let block = pos.map(|a| a.floor() as i32);
        let Some(level) = game.native_level(&town.territory.dimension) else {
            return;
        };
        if !town
            .territory
            .contains(&town.territory.dimension, block[0], block[2])
            || !game.native_level_is_loaded(level, block)
        {
            return;
        }
        let carrying = self
            .journal
            .pending()
            .is_some_and(|op| op.phase == Phase::Carrying);
        if carrying {
            if hand.as_ref() != Some(&ItemStack::new(world::ITEM, 1)) {
                self.held("cargo differs from physical hand; no compensation");
                return;
            }
            let Some((chest, approach)) = world::destination(game, &self.dir, town) else {
                return;
            };
            if !self.walk(game, town, id, pos, approach, tick, 1.0) {
                return;
            }
            if !world::chest_valid(game, town, &chest) {
                return;
            }
            let Some(before) = world::count(game, chest.pos) else {
                return;
            };
            if self
                .journal
                .transition(&self.dir, Phase::Depositing)
                .is_err()
            {
                self.held("journal refused deposit intent");
                return;
            }
            if !world::set_hand(game, id, None) {
                self.held("hand removal unconfirmed");
                return;
            }
            if game.save_entities(true).is_err() {
                self.held("hand persistence failed; no compensation");
                return;
            }
            let remainder =
                game.native_insert_item(chest.pos, Direction::Up, hand.expect("checked cargo"));
            if remainder.is_some() || world::count(game, chest.pos) != Some(before + 1) {
                self.held("deposit unconfirmed; reconcile inventories");
                return;
            }
            if game.save_level(true).is_err() {
                self.held("chest persistence failed");
                return;
            }
            if self
                .journal
                .transition(&self.dir, Phase::Delivered)
                .is_err()
            {
                self.held("delivery receipt persistence failed");
                return;
            }
            self.target = None;
            self.started = None;
            log::info!(
                "[kingdoms_alive] M2 lumberjack delivered job={} operations={}",
                self.journal.request.job,
                self.journal.operations.len()
            );
            return;
        }
        if hand.is_some() {
            self.held("worker hand occupied before assignment");
            return;
        }
        // Refuse harvesting while every approved warehouse is full or unavailable.
        if world::destination(game, &self.dir, town).is_none() {
            return;
        }
        if self.target.is_none() {
            self.target = self.journal.request.positions().find(|&p| {
                !self.journal.used(p)
                    && game.native_level_is_loaded(level, p)
                    && game.native_level_block(level, p).as_deref() == Some(world::ITEM)
            });
            self.started = None;
        }
        let Some(source) = self.target else {
            return;
        };
        if !game.native_level_is_loaded(level, source) {
            return;
        }
        if game.native_level_block(level, source).as_deref() != Some(world::ITEM) {
            self.target = None;
            self.started = None;
            return;
        }
        // The source must be in reach; an adjacent air tile is the actual path target.
        let Some(approach) = source[0].checked_sub(1).map(|x| [x, source[1], source[2]]) else {
            self.held("source coordinate overflow");
            return;
        };
        if !town
            .territory
            .contains(&town.territory.dimension, approach[0], approach[2])
            || !game.native_level_is_loaded(level, approach)
            || game.native_level_block(level, approach).as_deref() != Some("minecraft:air")
        {
            self.target = None;
            return;
        }
        if !self.walk(game, town, id, pos, approach, tick, 1.0) {
            return;
        }
        // Re-read before the one destructive call. Never reroll this position after an uncertain result.
        if game.native_level_block(level, source).as_deref() != Some(world::ITEM) {
            self.target = None;
            return;
        }
        if self.journal.begin(&self.dir, source).is_err() {
            self.held("journal refused harvest intent");
            return;
        }
        let Some(drops) = game.native_machine_destroy_block(source, None, true) else {
            self.held("harvest refused after intent");
            return;
        };
        // The helper can report drops even when replacement was vetoed. Do not
        // spawn those stacks while the source still exists: that would duplicate it.
        if game.native_level_block(level, source).as_deref() != Some("minecraft:air") {
            let _ = self.journal.transition(&self.dir, Phase::Held);
            self.held("source removal unconfirmed; no cargo or dropped copy created");
            return;
        }
        if drops != vec![ItemStack::new(world::ITEM, 1)] {
            for stack in drops {
                game.pop_resource(source, stack);
            }
            let _ = game.save_level(true);
            let _ = self.journal.transition(&self.dir, Phase::Held);
            self.held("nonstandard loot dropped physically at source; not rerolled");
            return;
        }
        // Persist source removal before creating physical cargo; crashes are held by Mining intent.
        if game.save_level(true).is_err() {
            self.held("source save failed");
            return;
        }
        if !world::set_hand(game, id, drops.into_iter().next()) || game.save_entities(true).is_err()
        {
            self.held("cargo persistence unconfirmed");
            return;
        }
        if self.journal.transition(&self.dir, Phase::Carrying).is_err() {
            self.held("cargo receipt persistence failed");
            return;
        }
        self.started = None;
        log::info!(
            "[kingdoms_alive] M2 lumberjack harvested job={} source={source:?}",
            self.journal.request.job
        );
    }
    fn walk(
        &mut self,
        game: &mut super::Game<'_>,
        town: &Settlement,
        id: u32,
        pos: [f64; 3],
        at: [i32; 3],
        tick: u64,
        reach: f64,
    ) -> bool {
        let [x, y, z] = at.map(f64::from);
        let target = [x + 0.5, y, z + 0.5];
        if pos
            .into_iter()
            .zip(target)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            <= reach * reach
        {
            game.native_mob_stop(&town.territory.dimension, id);
            return true;
        }
        if self.started.is_some_and(|t| tick.saturating_sub(t) >= 400) {
            game.native_mob_stop(&town.territory.dimension, id);
            self.held("navigation timeout; cargo retained");
            return false;
        }
        self.started.get_or_insert(tick);
        game.native_mob_navigate(&town.territory.dimension, id, target, 0, 1.0);
        false
    }
}
