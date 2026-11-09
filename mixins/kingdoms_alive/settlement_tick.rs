fn kingdoms_alive_settlement_tick(
    game: &mut crate::game::Game<'_>,
    context: &crate::mod_api::NativeLifecycleContext,
    phase: crate::mod_api::ServerTickPhase,
) {
    if matches!(
        phase,
        crate::mod_api::ServerTickPhase::ServerTickEnd {
            world_running: true
        }
    ) {
        crate::game::kingdoms_alive::tick(game, context);
    }
}
