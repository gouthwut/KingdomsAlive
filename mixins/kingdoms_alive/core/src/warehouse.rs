//! Explicit warehouse approval; inventory counts are observations, never saved stock.
use std::collections::BTreeSet;
use std::io;
use std::num::NonZeroU64;
use std::path::Path;

use serde::Deserialize;

use crate::settlement::{Settlement, Stockpile};

pub const FILE: &str = "kingdoms_alive_warehouses.json";
pub const MAX_CHESTS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chest {
    pub pos: [i32; 3],
    pub state: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub schema: u32,
    pub settlement: NonZeroU64,
    pub dimension: String,
    pub chests: Vec<Chest>,
}

impl Registry {
    pub fn open(dir: &Path, town: &Settlement) -> io::Result<Option<Self>> {
        let bytes = match std::fs::read(dir.join(FILE)) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let registry: Self = serde_json::from_slice(&bytes)?;
        registry.validate(town)?;
        Ok(Some(registry))
    }

    pub fn validate(&self, town: &Settlement) -> io::Result<()> {
        let mut seen = BTreeSet::new();
        if self.schema != 1
            || self.settlement != town.id
            || self.dimension != town.territory.dimension
            || self.chests.len() > MAX_CHESTS
            || self.chests.iter().any(|chest| {
                !seen.insert(chest.pos)
                    || !town
                        .territory
                        .contains(&self.dimension, chest.pos[0], chest.pos[2])
                    || chest.name.trim().is_empty()
                    || !chest.state.starts_with("minecraft:chest[")
                    || !chest.state.contains("type=single")
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid warehouse registry",
            ));
        }
        Ok(())
    }
}

/// Minimal M2 categories. Unrecognized items remain physical but are not counted as resources.
/// All arithmetic is checked: a failed observation is unknown, not a saturated stock value.
pub fn count<'a>(stacks: impl IntoIterator<Item = (&'a str, u32)>) -> Option<Stockpile> {
    let mut stock = Stockpile::default();
    for (item, amount) in stacks {
        let at = match item {
            "minecraft:bread" | "minecraft:carrot" | "minecraft:potato" | "minecraft:beetroot" => {
                &mut stock.food
            }
            "minecraft:oak_log"
            | "minecraft:birch_log"
            | "minecraft:spruce_log"
            | "minecraft:jungle_log"
            | "minecraft:acacia_log"
            | "minecraft:dark_oak_log"
            | "minecraft:mangrove_log"
            | "minecraft:cherry_log"
            | "minecraft:pale_oak_log" => &mut stock.wood,
            "minecraft:stone" | "minecraft:cobblestone" => &mut stock.stone,
            _ => continue,
        };
        *at = at.checked_add(amount)?;
    }
    Some(stock)
}
