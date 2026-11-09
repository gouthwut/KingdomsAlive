fn kingdoms_alive_settlement_observe(
    game: &mut crate::game::Game<'_>,
    dimension: &str,
    entity: u32,
    kind: &str,
) {
    if kind == "minecraft:villager" {
        crate::game::kingdoms_alive::observe(game, dimension, entity);
    }
}
