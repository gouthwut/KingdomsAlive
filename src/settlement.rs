use std::collections::BTreeSet;
use std::num::{NonZeroU16, NonZeroU64, NonZeroU128};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Territory {
    pub dimension: String,
    pub center: [i32; 3],
    pub radius: NonZeroU16,
}

impl Territory {
    /// Horizontal boundary for discovery. World edits require a separately approved site.
    pub fn contains(&self, dimension: &str, x: i32, z: i32) -> bool {
        let radius = u32::from(self.radius.get());
        self.dimension == dimension
            && self.center[0].abs_diff(x) <= radius
            && self.center[2].abs_diff(z) <= radius
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stockpile {
    pub food: u32,
    pub wood: u32,
    pub stone: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settlement {
    pub id: NonZeroU64,
    pub name: String,
    pub territory: Territory,
    pub residents: BTreeSet<NonZeroU128>,
    pub stockpile: Stockpile,
    pub housing_capacity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    Food,
    Housing,
    Wood,
    Stone,
}

impl Settlement {
    pub fn next_need(&self, wood_target: u32, stone_target: u32) -> Option<Need> {
        let population = self.residents.len();
        if population == 0 {
            None
        } else if (self.stockpile.food as usize) < population {
            Some(Need::Food)
        } else if (self.housing_capacity as usize) <= population {
            Some(Need::Housing)
        } else if self.stockpile.wood < wood_target {
            Some(Need::Wood)
        } else if self.stockpile.stone < stone_target {
            Some(Need::Stone)
        } else {
            None
        }
    }
}
