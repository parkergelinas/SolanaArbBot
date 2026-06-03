use common::AccountKey;
use rpc_client::{SubscriptionKind, SubscriptionSpec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountSubscription {
    pub account: AccountKey,
}

impl From<AccountSubscription> for SubscriptionSpec {
    fn from(subscription: AccountSubscription) -> Self {
        Self::new(SubscriptionKind::Account(subscription.account))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_subscription_maps_to_rpc_spec() {
        let spec = SubscriptionSpec::from(AccountSubscription { account: [3; 32] });

        assert_eq!(spec.kind, SubscriptionKind::Account([3; 32]));
    }
}
