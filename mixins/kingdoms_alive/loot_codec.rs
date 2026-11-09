// Pinned Horizon's item codec, wrapped in lossless typed NBT bytes, never Debug/JSON NBT.
use horizon_items::{item_nbt, items::ItemStack};
use horizon_world::nbt::{self, Limits};
use std::io;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

// Reject any stack that the engine's saved codec cannot round-trip exactly.
// This is evidence only, never permission to materialize a stack after a crash.
pub(super) fn serialize_loot(stacks: &[ItemStack]) -> io::Result<String> {
    let mut encoded = Vec::with_capacity(stacks.len());
    for stack in stacks {
        let tag = item_nbt::item_to_nbt(stack);
        let bytes = nbt::write_network(&tag, Limits::default())
            .map_err(|_| invalid("loot NBT encoding refused"))?;
        let (decoded, used) = nbt::read_network(&bytes, Limits::default())
            .map_err(|_| invalid("loot NBT decoding refused"))?;
        if used != bytes.len() || decoded != tag {
            return Err(invalid("loot NBT is not lossless"));
        }
        let restored = item_nbt::item_from_nbt(&decoded)
            .ok_or_else(|| invalid("loot stack cannot be decoded"))?;
        if restored != *stack || item_nbt::item_to_nbt(&restored) != tag {
            return Err(invalid("loot stack components cannot round-trip"));
        }
        encoded.push(bytes);
    }
    // Versioned envelope: each element is a complete network-root NBT byte array.
    serde_json::to_string(
        &serde_json::json!({"codec":"horizon-item-nbt-network-v1","stacks":encoded}),
    )
    .map_err(io::Error::other)
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_world::nbt::{JavaString, Tag};

    #[test]
    fn standard_and_empty_actual_results_are_distinct() {
        let empty = serialize_loot(&[]).unwrap();
        let standard = serialize_loot(&[ItemStack::new("minecraft:oak_log", 1)]).unwrap();
        assert_ne!(empty, standard);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&empty).unwrap()["stacks"],
            serde_json::json!([])
        );
    }
    #[test]
    fn unknown_components_retain_nbt_types_and_utf16() {
        let mut stack = ItemStack::new("minecraft:oak_log", 2);
        let mut extra = horizon_items::item_data::StackData::default();
        extra.other.push((
            "test:data".into(),
            Tag::compound([
                ("byte", Tag::Byte(7)),
                ("int", Tag::Int(7)),
                ("text", Tag::String(JavaString(vec![0xd800]))),
                ("array", Tag::LongArray(vec![i64::MAX])),
            ]),
        ));
        stack.extra = Some(Box::new(extra));
        let encoded = serialize_loot(&[stack.clone()]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        let bytes: Vec<u8> = serde_json::from_value(value["stacks"][0].clone()).unwrap();
        let (tag, used) = nbt::read_network(&bytes, Limits::default()).unwrap();
        assert_eq!(used, bytes.len());
        assert_eq!(item_nbt::item_from_nbt(&tag), Some(stack));
    }
    #[test]
    fn unrepresentable_runtime_counts_and_float_normalization_are_refused() {
        for count in [0, 100, 1000, u32::MAX] {
            assert!(serialize_loot(&[ItemStack::new("minecraft:oak_log", count)]).is_err());
        }
        let mut stack = ItemStack::new("minecraft:oak_log", 1);
        let mut extra = horizon_items::item_data::StackData::default();
        extra
            .other
            .push(("test:data".into(), Tag::Float((-0.0_f32).to_bits())));
        stack.extra = Some(Box::new(extra));
        assert!(serialize_loot(&[stack]).is_err());
    }
}
