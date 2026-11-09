//! Независимые тесты публичного контракта `kingdoms_alive_core::recovery`.
//!
//! Проверяется только модель сверки из согласованного контракта: типы
//! `Phase`, `Observation`, `Snapshot`, `Operation`, `RecoveryDecision` и
//! чистая функция `reconcile`. Эти тесты НЕ доказывают crash consistency
//! Horizon, целостность дискового журнала или поведение адаптера: строки
//! снимков здесь — непрозрачные канонические представления, а не схема
//! сохранений. Реальные аварийные проверки мира перечислены в
//! docs/qa/M2_RECOVERY_CASES.md и выполняются отдельно.
//!
//! Решения `reconcile` диагностические: ни одно из них не является
//! разрешением повторной добычи, повторной вставки или выдачи предметов.

use kingdoms_alive_core::recovery::{
    Observation, Operation, Phase, RecoveryDecision, Snapshot, reconcile,
};

/// Канонические строки снимков — произвольные непрозрачные метки.
const BLOCK_UNMINED: &str = "block=oak_log@10,70,20";
const BLOCK_MINED: &str = "block=air@10,70,20";
const CARGO_EMPTY: &str = "cargo=offhand:empty";
const CARGO_ONE_LOG: &str = "cargo=offhand:oak_log#1";
const WAREHOUSE_EMPTY: &str = "warehouse=chest@12,70,20:slots=empty";
const WAREHOUSE_ONE_LOG: &str = "warehouse=chest@12,70,20:slots=oak_log#1";

fn obs(value: &'static str) -> Observation {
    Observation::Available(value.to_string())
}

fn snapshot(block: Observation, cargo: Observation, warehouse: Observation) -> Snapshot {
    Snapshot {
        block,
        cargo,
        warehouse,
    }
}

/// Исходное состояние: блок на месте, рука и сундук пусты.
fn before() -> Snapshot {
    snapshot(obs(BLOCK_UNMINED), obs(CARGO_EMPTY), obs(WAREHOUSE_EMPTY))
}

/// Конечное состояние: блок добыт, бревно в руке, сундук ещё пуст
/// (фаза «груз → сундук» описывается отдельной операцией).
fn after() -> Snapshot {
    snapshot(obs(BLOCK_MINED), obs(CARGO_ONE_LOG), obs(WAREHOUSE_EMPTY))
}

fn operation(phase: Phase, loot: Option<&str>, before: Snapshot, after: Snapshot) -> Operation {
    Operation {
        id: "op-1".to_string(),
        phase,
        actual_loot: loot.map(str::to_string),
        before,
        after,
    }
}

/// Обычная операция добычи с известным непустым лутом.
fn mining(phase: Phase) -> Operation {
    operation(phase, Some("oak_log#1"), before(), after())
}

#[test]
fn confirmed_returns_already_confirmed_even_when_world_is_unavailable() {
    let observed = snapshot(
        Observation::Unavailable,
        Observation::Unavailable,
        Observation::Unavailable,
    );
    let mut op = mining(Phase::Confirmed);
    assert_eq!(
        reconcile(&op, &observed),
        RecoveryDecision::AlreadyConfirmed
    );
    // Confirmed имеет приоритет и над неизвестным лутом, одинаковыми
    // снимками, недоступными сохранёнными endpoints одновременно.
    op.actual_loot = None;
    op.before = observed.clone();
    op.after = observed.clone();
    assert_eq!(
        reconcile(&op, &observed),
        RecoveryDecision::AlreadyConfirmed
    );
}

#[test]
fn confirmed_returns_already_confirmed_for_every_observed_state() {
    // Подтверждённая операция никогда не разрешает повторное применение,
    // что бы ни показывал мир: исходное, конечное или промежуточное.
    for observed in [
        before(),
        after(),
        snapshot(obs(BLOCK_MINED), obs(CARGO_EMPTY), obs(WAREHOUSE_EMPTY)),
    ] {
        assert_eq!(
            reconcile(&mining(Phase::Confirmed), &observed),
            RecoveryDecision::AlreadyConfirmed
        );
    }
}

#[test]
fn prepared_matching_before_yields_observed_before() {
    let op = mining(Phase::Prepared);
    assert_eq!(reconcile(&op, &before()), RecoveryDecision::ObservedBefore);
}

#[test]
fn prepared_matching_after_yields_observed_after() {
    let op = mining(Phase::Prepared);
    assert_eq!(reconcile(&op, &after()), RecoveryDecision::ObservedAfter);
}

#[test]
fn executed_distinguishes_before_and_after_the_same_way() {
    let op = mining(Phase::Executed);
    assert_eq!(reconcile(&op, &before()), RecoveryDecision::ObservedBefore);
    assert_eq!(reconcile(&op, &after()), RecoveryDecision::ObservedAfter);
}

#[test]
fn mixed_observed_state_yields_hold() {
    // Промежуточная запись: блок изменён, но груз ещё не записан.
    // Это не «исходное» и не «конечное» состояние — только Hold.
    for phase in [Phase::Prepared, Phase::Executed] {
        let op = mining(phase);
        for partial in [
            snapshot(obs(BLOCK_MINED), obs(CARGO_EMPTY), obs(WAREHOUSE_EMPTY)),
            snapshot(obs(BLOCK_UNMINED), obs(CARGO_ONE_LOG), obs(WAREHOUSE_EMPTY)),
        ] {
            assert_eq!(reconcile(&op, &partial), RecoveryDecision::Hold);
        }
    }
}

#[test]
fn uncertain_phase_always_holds_even_on_exact_match() {
    let op = mining(Phase::Uncertain);
    assert_eq!(reconcile(&op, &before()), RecoveryDecision::Hold);
    assert_eq!(reconcile(&op, &after()), RecoveryDecision::Hold);
}

#[test]
fn unknown_loot_holds_even_on_exact_match() {
    // Неизвестный фактический результат добычи запрещает автоматическое
    // разрешение, даже если мир точно совпал с ожидаемым снимком.
    for phase in [Phase::Prepared, Phase::Executed] {
        let op = operation(phase, None, before(), after());
        assert_eq!(reconcile(&op, &before()), RecoveryDecision::Hold);
        assert_eq!(reconcile(&op, &after()), RecoveryDecision::Hold);
    }
}

#[test]
fn empty_loot_is_a_known_result_and_reconciles_normally() {
    // Some(пусто) — известный результат. Модель не разбирает формат:
    // проверяем и пустую строку, и условную сериализацию пустого списка.
    let no_drops = snapshot(obs(BLOCK_MINED), obs(CARGO_EMPTY), obs(WAREHOUSE_EMPTY));
    for phase in [Phase::Prepared, Phase::Executed] {
        for loot in ["", "[]"] {
            let op = operation(phase.clone(), Some(loot), before(), no_drops.clone());
            assert_eq!(reconcile(&op, &before()), RecoveryDecision::ObservedBefore);
            assert_eq!(reconcile(&op, &no_drops), RecoveryDecision::ObservedAfter);
        }
    }
}

#[test]
fn unavailable_in_any_snapshot_component_holds() {
    // Все 9 мест: block/cargo/warehouse в before/after/observed.
    // Проверяем оба точных совпадения: недоступность даже НЕ совпавшего
    // журнального снимка имеет приоритет перед распознаванием другого.
    for phase in [Phase::Prepared, Phase::Executed] {
        for location in 0..3 {
            for component in 0..3 {
                for observed in [before(), after()] {
                    let mut op = mining(phase.clone());
                    let mut observed = observed;
                    let target = match location {
                        0 => &mut op.before,
                        1 => &mut op.after,
                        _ => &mut observed,
                    };
                    *match component {
                        0 => &mut target.block,
                        1 => &mut target.cargo,
                        _ => &mut target.warehouse,
                    } = Observation::Unavailable;
                    assert_eq!(reconcile(&op, &observed), RecoveryDecision::Hold);
                }
            }
        }
    }
}

#[test]
fn equal_unavailable_observations_are_not_available_evidence() {
    let missing = snapshot(
        Observation::Unavailable,
        obs(CARGO_EMPTY),
        obs(WAREHOUSE_EMPTY),
    );
    let op = operation(Phase::Executed, Some("oak_log#1"), missing.clone(), after());
    assert_eq!(reconcile(&op, &missing), RecoveryDecision::Hold);
}

#[test]
fn full_state_is_opaque_not_just_item_counts() {
    let op = mining(Phase::Executed);
    // Количество то же; другие компоненты стека, слот или блок — конфликт.
    let mut different_cargo = after();
    different_cargo.cargo = obs("cargo=offhand:oak_log#1;custom_name=player");
    let warehouse_before = snapshot(
        obs(BLOCK_MINED),
        obs(CARGO_EMPTY),
        obs("warehouse=chest@12,70,20:slot0=oak_log#1"),
    );
    let warehouse_after = snapshot(
        obs(BLOCK_MINED),
        obs(CARGO_EMPTY),
        obs("warehouse=chest@12,70,20:slot0=oak_log#2"),
    );
    let warehouse_op = operation(
        Phase::Executed,
        Some("oak_log#1"),
        warehouse_before,
        warehouse_after,
    );
    let different_slot = snapshot(
        obs(BLOCK_MINED),
        obs(CARGO_EMPTY),
        obs("warehouse=chest@12,70,20:slot26=oak_log#1"),
    );
    assert_eq!(
        reconcile(&warehouse_op, &different_slot),
        RecoveryDecision::Hold
    );
    let mut different_block = before();
    different_block.block = obs("block=oak_log[axis=x]@10,70,20");
    for observed in [different_cargo, different_slot, different_block] {
        assert_eq!(reconcile(&op, &observed), RecoveryDecision::Hold);
    }
    // Даже пустая Available-строка не означает Unavailable.
    let mut empty_string_before = before();
    empty_string_before.block = obs("");
    let op = operation(
        Phase::Prepared,
        Some("[]"),
        empty_string_before.clone(),
        after(),
    );
    assert_eq!(
        reconcile(&op, &empty_string_before),
        RecoveryDecision::ObservedBefore
    );
}

#[test]
fn identical_before_and_after_holds_even_when_observed_matches() {
    // Одинаковые снимки не несут информации о прогрессе: точное совпадение
    // с ними ничего не разрешает.
    for phase in [Phase::Prepared, Phase::Executed] {
        let same = before();
        let op = operation(phase, Some("oak_log#1"), same.clone(), same);
        assert_eq!(reconcile(&op, &before()), RecoveryDecision::Hold);
    }
}

#[test]
fn intermediate_deposit_records_hold_without_replay_or_compensation() {
    let deposited = snapshot(obs(BLOCK_MINED), obs(CARGO_EMPTY), obs(WAREHOUSE_ONE_LOG));
    for phase in [Phase::Prepared, Phase::Executed] {
        // Отдельная операция «рука → сундук»: исходное и конечное
        // состояния различимы, обе частичные перестановки блокируются.
        let op = operation(phase, Some("oak_log#1"), after(), deposited.clone());
        assert_eq!(reconcile(&op, &after()), RecoveryDecision::ObservedBefore);
        assert_eq!(reconcile(&op, &deposited), RecoveryDecision::ObservedAfter);
        for partial in [
            snapshot(obs(BLOCK_MINED), obs(CARGO_EMPTY), obs(WAREHOUSE_EMPTY)),
            snapshot(obs(BLOCK_MINED), obs(CARGO_ONE_LOG), obs(WAREHOUSE_ONE_LOG)),
        ] {
            assert_eq!(reconcile(&op, &partial), RecoveryDecision::Hold);
        }
    }
}

#[test]
fn repeated_reconcile_is_stable_and_does_not_mutate_inputs() {
    for phase in [Phase::Prepared, Phase::Executed] {
        let op = mining(phase);
        let original = op.clone();
        for (observed, expected) in [
            (before(), RecoveryDecision::ObservedBefore),
            (after(), RecoveryDecision::ObservedAfter),
            (before(), RecoveryDecision::ObservedBefore),
        ] {
            let saved = observed.clone();
            for _ in 0..3 {
                assert_eq!(reconcile(&op, &observed), expected);
                // Диагноз не подтверждает операцию, не расходует лут
                // и не меняет переданные данные при повторной сверке.
                assert_eq!(op, original);
                assert_eq!(observed, saved);
            }
        }
    }
}

#[test]
fn observed_before_and_after_are_diagnostics_not_issuance_permissions() {
    let op = mining(Phase::Prepared);
    let before_decision = reconcile(&op, &before());
    let after_decision = reconcile(&op, &after());
    // Исчерпывающее сопоставление публичного enum: вариантов выдачи,
    // повтора или компенсации нет. Это не тест игрового вызывающего кода.
    let diagnostic_only = |decision: RecoveryDecision| match decision {
        RecoveryDecision::AlreadyConfirmed => "confirmed; no replay",
        RecoveryDecision::Hold => "held; no replay",
        RecoveryDecision::ObservedBefore => "before; no replay",
        RecoveryDecision::ObservedAfter => "after; no replay",
    };
    assert_eq!(diagnostic_only(before_decision), "before; no replay");
    assert_eq!(diagnostic_only(after_decision), "after; no replay");
    assert_eq!(op.phase, Phase::Prepared);
    let confirmed = Operation {
        phase: Phase::Confirmed,
        ..op
    };
    for observed in [before(), after()] {
        assert_eq!(
            reconcile(&confirmed, &observed),
            RecoveryDecision::AlreadyConfirmed
        );
    }
}
