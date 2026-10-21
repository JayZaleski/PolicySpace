//! Natural resources: resource types and the deposits of them on the map.
//!
//! - A **resource type** (`[[resource]]` in `rules/`) is a kind of natural resource: the
//!   one good it produces, and how much one deposit produces per period. Several resource
//!   types may produce the same good. The engine doesn't require that good to be unmovable
//!   or to perish, though settings usually make it so.
//! - A **deposit** (`[[deposit]]` in `start/`) is one unit of a resource type on a hex. A
//!   setting entry can place several identical deposits on a hex (`count`); each is its own
//!   deposit with its own ID and its own single owner. A hex may hold deposits of several
//!   types.
//! - Deposits have no names. They are identified in setting files by hex and resource
//!   type; [`DepositId`]s are valid only within the [`Deposits`] that produced them.
//! - A deposit has at most one owner, a business ([`BusinessId`]). Ownership is written on
//!   the business in the setting (`start/`), which claims its deposits when the setting
//!   loads, and does not change during play yet.
//! - Each deposit emits its type's `output` every turn (see the [`world`](crate::world)
//!   module): into its owner's stockpile if it has an owner, otherwise into its hex's
//!   stockpile. There is no depletion or regeneration (later).
//!
//! This module knows nothing about files: the setting loader builds [`ResourceTypes`] and
//! [`Deposits`] from a checked setting, and everything else only reads them.

use crate::businesses::{BusinessId, BusinessKind};
use crate::goods::GoodId;
use crate::id::{Id, IdKind, IdVec, NamedList};
use crate::map::{HexId, HexKind};
use crate::quantity::Quantity;

/// Marker kind for resource types.
pub enum ResourceKind {}

impl IdKind for ResourceKind {
    const LABEL: &'static str = "resource";
}

/// The ID of a resource type.
pub type ResourceId = Id<ResourceKind>;

/// One resource type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceType {
    /// The good one deposit produces.
    pub produces: GoodId,
    /// How much of it one deposit produces per period. Always greater than 0.
    pub output: Quantity,
}

/// Every resource type in a setting, in load order (which is also ID order).
pub type ResourceTypes = NamedList<ResourceKind, ResourceType>;

/// Marker kind for deposits.
pub enum DepositKind {}

impl IdKind for DepositKind {
    const LABEL: &'static str = "deposit";
}

/// The ID of a deposit.
pub type DepositId = Id<DepositKind>;

/// One deposit: one unit of a resource type on a hex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Deposit {
    /// Its resource type.
    pub resource: ResourceId,
    /// The hex it sits on.
    pub hex: HexId,
    /// The business that owns it, if any. Set by the loader from the owner's `[[business]]`
    /// entry; nothing changes it during play yet. An owned deposit emits into its owner's
    /// stockpile, an unowned one into its hex's.
    pub owner: Option<BusinessId>,
}

/// Every deposit in a setting, in load order (which is also ID order).
///
/// `Deposits::default()` is empty and was built with no map and no businesses, so
/// [`on_hex`](Self::on_hex) and [`owned_by`](Self::owned_by) panic on it for any hex or
/// business.
#[derive(Debug, Default)]
pub struct Deposits {
    /// One entry per deposit, by `DepositId`.
    deposits: IdVec<DepositKind, Deposit>,
    /// One entry per hex, by `HexId`: the deposits on each hex, in ID order.
    by_hex: IdVec<HexKind, Vec<DepositId>>,
    /// One entry per business, by `BusinessId`: the deposits each owns, in ID order. A
    /// deposit with no owner is in none of them.
    by_owner: IdVec<BusinessKind, Vec<DepositId>>,
}

impl Deposits {
    /// Builds the list, giving IDs 0, 1, 2, ... in the order given. `hex_count` is the
    /// number of hexes in the map the deposits sit on and `business_count` the number of
    /// businesses that may own them. Crate-private: only the setting loader builds deposits.
    ///
    /// # Panics
    /// Panics if a deposit's hex index is `>= hex_count`, if a deposit's owner index is
    /// `>= business_count` (`"{owner:?} is not from a list of {business_count}
    /// businesses"`), or if there are more than `u32::MAX` deposits.
    pub(crate) fn from_deposits(
        deposits: Vec<Deposit>,
        hex_count: usize,
        business_count: usize,
    ) -> Self {
        let deposits = IdVec::from_vec(deposits);
        let mut by_hex: IdVec<HexKind, Vec<DepositId>> =
            IdVec::from_vec(vec![Vec::new(); hex_count]);
        let mut by_owner: IdVec<BusinessKind, Vec<DepositId>> =
            IdVec::from_vec(vec![Vec::new(); business_count]);
        for (id, deposit) in deposits.iter() {
            assert!(
                deposit.hex.index() < hex_count,
                "{:?} is not from a map of {hex_count} hexes",
                deposit.hex
            );
            if let Some(owner) = deposit.owner {
                assert!(
                    owner.index() < business_count,
                    "{owner:?} is not from a list of {business_count} businesses"
                );
                by_owner.get_mut(owner).push(id);
            }
            by_hex.get_mut(deposit.hex).push(id);
        }
        Deposits {
            deposits,
            by_hex,
            by_owner,
        }
    }

    /// Number of deposits.
    pub fn len(&self) -> usize {
        self.deposits.len()
    }

    /// True if the setting has no deposits.
    pub fn is_empty(&self) -> bool {
        self.deposits.is_empty()
    }

    /// The deposit `id`.
    ///
    /// # Panics
    /// Panics if `id` isn't from this list.
    pub fn get(&self, id: DepositId) -> &Deposit {
        self.deposits.get(id)
    }

    /// Every deposit as (ID, data), in load order.
    pub fn iter(&self) -> impl Iterator<Item = (DepositId, &Deposit)> + '_ {
        self.deposits.iter()
    }

    /// The deposits on `hex`, in ID order; empty if it has none.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these deposits were built with.
    pub fn on_hex(&self, hex: HexId) -> &[DepositId] {
        self.by_hex.get(hex)
    }

    /// The deposits owned by `business`, in ID order; none if it owns none.
    ///
    /// # Panics
    /// Panics if `business` isn't from the businesses these deposits were built with (and
    /// on `Deposits::default()`, for any business).
    pub fn owned_by(&self, business: BusinessId) -> impl Iterator<Item = DepositId> + '_ {
        self.by_owner.get(business).iter().copied()
    }

    /// The deposits of `resource` on `hex`, in ID order.
    ///
    /// # Panics
    /// Panics if `hex` isn't from the map these deposits were built with.
    pub fn of_type_on_hex(
        &self,
        hex: HexId,
        resource: ResourceId,
    ) -> impl Iterator<Item = DepositId> + '_ {
        self.on_hex(hex)
            .iter()
            .copied()
            .filter(move |id| self.deposits.get(*id).resource == resource)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::NameRegistry;

    fn resource_ids(count: usize) -> Vec<ResourceId> {
        let mut names: NameRegistry<ResourceKind> = NameRegistry::new();
        (0..count)
            .map(|i| names.register(&format!("r{i}")).unwrap())
            .collect()
    }

    fn hex_ids(count: usize) -> Vec<HexId> {
        let mut names: NameRegistry<HexKind> = NameRegistry::new();
        (0..count)
            .map(|i| names.register(&format!("h{i}")).unwrap())
            .collect()
    }

    /// Three hexes; deposits: 0 r0@h0, 1 r1@h0, 2 r0@h2, 3 r0@h0.
    fn sample_deposits() -> Deposits {
        let r = resource_ids(2);
        let h = hex_ids(3);
        Deposits::from_deposits(
            vec![
                Deposit {
                    resource: r[0],
                    hex: h[0],
                    owner: None,
                },
                Deposit {
                    resource: r[1],
                    hex: h[0],
                    owner: None,
                },
                Deposit {
                    resource: r[0],
                    hex: h[2],
                    owner: None,
                },
                Deposit {
                    resource: r[0],
                    hex: h[0],
                    owner: None,
                },
            ],
            3,
            0,
        )
    }

    fn indexes(ids: &[DepositId]) -> Vec<usize> {
        ids.iter().map(|id| id.index()).collect()
    }

    #[test]
    fn deposits_get_sequential_ids_in_the_order_given() {
        let deposits = sample_deposits();
        assert_eq!(deposits.len(), 4);
        assert!(!deposits.is_empty());
        let listed: Vec<(usize, usize, usize)> = deposits
            .iter()
            .map(|(id, d)| (id.index(), d.resource.index(), d.hex.index()))
            .collect();
        assert_eq!(listed, vec![(0, 0, 0), (1, 1, 0), (2, 0, 2), (3, 0, 0)]);
        let (id, _) = deposits.iter().nth(2).unwrap();
        assert_eq!(deposits.get(id).hex.index(), 2);
    }

    #[test]
    fn on_hex_lists_deposits_in_id_order() {
        let deposits = sample_deposits();
        let h = hex_ids(3);
        assert_eq!(indexes(deposits.on_hex(h[0])), vec![0, 1, 3]);
        assert!(deposits.on_hex(h[1]).is_empty());
        assert_eq!(indexes(deposits.on_hex(h[2])), vec![2]);
    }

    #[test]
    fn of_type_on_hex_filters_by_resource_in_id_order() {
        let deposits = sample_deposits();
        let h = hex_ids(3);
        let r = resource_ids(2);
        let of = |hex, resource| -> Vec<usize> {
            deposits
                .of_type_on_hex(hex, resource)
                .map(|id| id.index())
                .collect()
        };
        assert_eq!(of(h[0], r[0]), vec![0, 3]);
        assert_eq!(of(h[0], r[1]), vec![1]);
        assert!(of(h[1], r[0]).is_empty());
        assert!(of(h[2], r[1]).is_empty());
    }

    #[test]
    fn owned_by_lists_only_that_owners_deposits_in_id_order() {
        let mut names: NameRegistry<BusinessKind> = NameRegistry::new();
        let owners = [
            names.register("b0").unwrap(),
            names.register("b1").unwrap(),
            names.register("b2").unwrap(),
        ];
        let r = resource_ids(2);
        let h = hex_ids(1);
        let deposit = |resource: ResourceId, owner: Option<BusinessId>| Deposit {
            resource,
            hex: h[0],
            owner,
        };
        let deposits = Deposits::from_deposits(
            vec![
                deposit(r[0], Some(owners[1])),
                deposit(r[0], None),
                deposit(r[1], Some(owners[0])),
                deposit(r[1], Some(owners[1])),
            ],
            1,
            3,
        );
        let owned =
            |b: BusinessId| -> Vec<usize> { deposits.owned_by(b).map(|id| id.index()).collect() };
        assert_eq!(owned(owners[0]), vec![2]);
        assert_eq!(owned(owners[1]), vec![0, 3]);
        assert!(owned(owners[2]).is_empty());
        // A fixture deposit starts with no owner.
        assert!(sample_deposits().iter().all(|(_, d)| d.owner.is_none()));
    }

    #[test]
    fn the_default_deposit_list_is_empty() {
        let deposits = Deposits::default();
        assert_eq!(deposits.len(), 0);
        assert!(deposits.is_empty());
        assert_eq!(deposits.iter().count(), 0);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn deposit_get_panics_on_a_foreign_id() {
        let mut names: NameRegistry<DepositKind> = NameRegistry::new();
        names.register("a").unwrap();
        let foreign = names.register("b").unwrap();
        Deposits::default().get(foreign);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn on_hex_panics_on_a_hex_beyond_the_map() {
        sample_deposits().on_hex(hex_ids(4)[3]);
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn on_hex_panics_on_the_default_list() {
        Deposits::default().on_hex(hex_ids(1)[0]);
    }

    #[test]
    #[should_panic(expected = "is not from a map of 2 hexes")]
    fn from_deposits_panics_on_a_hex_beyond_hex_count() {
        let r = resource_ids(1);
        let h = hex_ids(3);
        Deposits::from_deposits(
            vec![Deposit {
                resource: r[0],
                hex: h[2],
                owner: None,
            }],
            2,
            0,
        );
    }

    #[test]
    #[should_panic(expected = "is not from a list of 2 businesses")]
    fn from_deposits_panics_on_an_owner_beyond_business_count() {
        let mut names: NameRegistry<BusinessKind> = NameRegistry::new();
        let owners = [
            names.register("b0").unwrap(),
            names.register("b1").unwrap(),
            names.register("b2").unwrap(),
        ];
        let r = resource_ids(1);
        let h = hex_ids(1);
        Deposits::from_deposits(
            vec![Deposit {
                resource: r[0],
                hex: h[0],
                owner: Some(owners[2]),
            }],
            1,
            2,
        );
    }

    #[test]
    #[should_panic(expected = "is not from this list")]
    fn owned_by_panics_on_the_default_list() {
        let mut names: NameRegistry<BusinessKind> = NameRegistry::new();
        let owner = names.register("b0").unwrap();
        let _ = Deposits::default().owned_by(owner);
    }
}
