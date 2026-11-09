// Reconciliation is diagnostic only: no result authorizes replay or item creation.
// The caller must isolate unfinished operations and stop on read or save failures.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Prepared,
    Executed,
    Confirmed,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    Unavailable,
    Available(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub block: Observation,
    pub cargo: Observation,
    pub warehouse: Observation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
    pub phase: Phase,
    pub actual_loot: Option<String>,
    pub before: Snapshot,
    pub after: Snapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryDecision {
    AlreadyConfirmed,
    Hold,
    ObservedBefore,
    ObservedAfter,
}

impl Snapshot {
    fn is_available(&self) -> bool {
        matches!(&self.block, Observation::Available(_))
            && matches!(&self.cargo, Observation::Available(_))
            && matches!(&self.warehouse, Observation::Available(_))
    }
}

/// Classifies complete canonical observations without changing operation or world state.
/// A classification is not permission to repeat or compensate an operation.
pub fn reconcile(operation: &Operation, observed: &Snapshot) -> RecoveryDecision {
    if operation.phase == Phase::Confirmed {
        return RecoveryDecision::AlreadyConfirmed;
    }

    if operation.phase == Phase::Uncertain
        || operation.actual_loot.is_none()
        || !operation.before.is_available()
        || !operation.after.is_available()
        || !observed.is_available()
        || operation.before == operation.after
    {
        return RecoveryDecision::Hold;
    }

    if observed == &operation.before {
        RecoveryDecision::ObservedBefore
    } else if observed == &operation.after {
        RecoveryDecision::ObservedAfter
    } else {
        RecoveryDecision::Hold
    }
}
