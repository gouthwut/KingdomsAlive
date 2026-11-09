fn kingdoms_alive_settlement_listen(listened: &mut bool) {
    *listened |= crate::game::kingdoms_alive::observing();
}
