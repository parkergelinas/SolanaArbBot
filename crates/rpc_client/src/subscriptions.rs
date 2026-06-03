use common::{AccountKey, ProgramId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptionKind {
    ProgramAccounts(ProgramId),
    Account(AccountKey),
    Logs(ProgramId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubscriptionSpec {
    pub kind: SubscriptionKind,
}

impl SubscriptionSpec {
    pub const fn new(kind: SubscriptionKind) -> Self {
        Self { kind }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_spec_preserves_kind() {
        let kind = SubscriptionKind::Account([7; 32]);
        let spec = SubscriptionSpec::new(kind);

        assert_eq!(spec.kind, kind);
    }
}
