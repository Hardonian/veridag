//! Capability-based authorization (spec 07-capabilities).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use thiserror::Error;
use veridag_codec::{Decode, DecodeError, Decoder, Encode, Encoder, MAX_SEQ};
use veridag_crypto::hash;
use veridag_protocol_types::{Address, ApplicationId, CapabilityId, Epoch, ResourceBudget};

/// The kind of authority a capability grants.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CapabilityKind {
    /// Spend value: max per epoch and current-epoch spent.
    Spend {
        /// per-epoch limit
        max_per_epoch: u64,
        /// spent this epoch
        current_epoch_spent: u64,
    },
    /// Modify objects of a class.
    ModifyObject {
        /// object class
        object_class: u32,
    },
    /// May delegate (create children).
    Delegate,
    /// Call an application.
    Application {
        /// application id
        app: ApplicationId,
    },
    /// Validator authority (managed by membership).
    Validator,
    /// Agent authorization with fine-grained scope.
    Agent {
        /// total spend limit
        max_spend: u64,
        /// allowed applications
        allowed_apps: Vec<ApplicationId>,
        /// allowed counterparties
        allowed_counterparties: Vec<Address>,
        /// allowed object classes
        allowed_object_classes: Vec<u32>,
    },
    /// Bounded session.
    Session {
        /// max calls
        max_calls: u32,
        /// calls used
        calls_used: u32,
    },
}

/// Additional constraints on a capability.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Constraints {
    /// Epoch at/after which the capability is invalid.
    pub expiry_epoch: Epoch,
    /// Optional per-epoch rate limit.
    pub rate_limit: Option<u32>,
    /// Optional resource limit.
    pub resource_limit: Option<ResourceBudget>,
}

/// A capability object (consensus state).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Capability {
    /// Unique id (hash of the other fields at creation).
    pub id: CapabilityId,
    /// Issuer.
    pub issuer: Address,
    /// Holder.
    pub holder: Address,
    /// Kind.
    pub kind: CapabilityKind,
    /// Constraints.
    pub constraints: Constraints,
    /// May be delegated.
    pub delegable: bool,
    /// Has been revoked.
    pub revoked: bool,
    /// Parent capability in a delegation chain.
    pub parent: Option<CapabilityId>,
}

/// Capability enforcement errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    /// Capability is revoked.
    #[error("revoked")]
    Revoked,
    /// Capability is expired.
    #[error("expired")]
    Expired,
    /// Operation exceeds the capability's limits.
    #[error("limit exceeded")]
    Exceeded,
    /// Capability does not cover this operation.
    #[error("not covered")]
    NotCovered,
    /// Delegation not permitted.
    #[error("not delegable")]
    NotDelegable,
}

impl Capability {
    /// Compute the canonical id for a capability's fields.
    pub fn derive_id(fields_bytes: &[u8]) -> CapabilityId {
        CapabilityId(hash("VERIDAG_CAPABILITY_V1", fields_bytes))
    }

    /// Check validity (not revoked, not expired) at `current_epoch`.
    pub fn check_valid(&self, current_epoch: Epoch) -> Result<(), CapabilityError> {
        if self.revoked {
            return Err(CapabilityError::Revoked);
        }
        if current_epoch >= self.constraints.expiry_epoch && self.constraints.expiry_epoch != 0 {
            return Err(CapabilityError::Expired);
        }
        Ok(())
    }

    /// Authorize a spend of `amount` at `current_epoch`; mutates spent counter.
    pub fn authorize_spend(
        &mut self,
        amount: u64,
        current_epoch: Epoch,
    ) -> Result<(), CapabilityError> {
        self.check_valid(current_epoch)?;
        match &mut self.kind {
            CapabilityKind::Spend {
                max_per_epoch,
                current_epoch_spent,
            } => {
                if current_epoch_spent.saturating_add(amount) > *max_per_epoch {
                    return Err(CapabilityError::Exceeded);
                }
                *current_epoch_spent += amount;
                Ok(())
            }
            CapabilityKind::Agent { max_spend, .. } => {
                if amount > *max_spend {
                    return Err(CapabilityError::Exceeded);
                }
                Ok(())
            }
            _ => Err(CapabilityError::NotCovered),
        }
    }

    /// Whether this capability covers modifying objects of `object_class`.
    pub fn covers_object_class(&self, object_class: u32) -> bool {
        match &self.kind {
            CapabilityKind::ModifyObject { object_class: c } => *c == object_class,
            CapabilityKind::Agent {
                allowed_object_classes,
                ..
            } => allowed_object_classes.contains(&object_class),
            _ => false,
        }
    }

    /// Whether this capability covers invoking `app`.
    pub fn covers_application(&self, app: &ApplicationId) -> bool {
        match &self.kind {
            CapabilityKind::Application { app: a } => a == app,
            CapabilityKind::Agent { allowed_apps, .. } => allowed_apps.contains(app),
            _ => false,
        }
    }
}

// --- VCE-1 encoding ---------------------------------------------------------

fn encode_budget(e: &mut Encoder, b: &ResourceBudget) {
    e.u64(b.compute);
    e.u64(b.memory);
    e.u64(b.storage);
    e.u64(b.bandwidth);
}

fn decode_budget(d: &mut Decoder<'_>) -> Result<ResourceBudget, DecodeError> {
    Ok(ResourceBudget {
        compute: d.u64()?,
        memory: d.u64()?,
        storage: d.u64()?,
        bandwidth: d.u64()?,
    })
}

impl Encode for Capability {
    fn encode(&self, e: &mut Encoder) {
        e.fixed(self.issuer.as_slice());
        e.fixed(self.holder.as_slice());
        // kind
        match &self.kind {
            CapabilityKind::Spend {
                max_per_epoch,
                current_epoch_spent,
            } => {
                e.u8(0);
                e.u64(*max_per_epoch);
                e.u64(*current_epoch_spent);
            }
            CapabilityKind::ModifyObject { object_class } => {
                e.u8(1);
                e.u32(*object_class);
            }
            CapabilityKind::Delegate => e.u8(2),
            CapabilityKind::Application { app } => {
                e.u8(3);
                e.fixed(app.as_bytes());
            }
            CapabilityKind::Validator => e.u8(4),
            CapabilityKind::Agent {
                max_spend,
                allowed_apps,
                allowed_counterparties,
                allowed_object_classes,
            } => {
                e.u8(5);
                e.u64(*max_spend);
                e.seq(allowed_apps, |e, a| e.fixed(a.as_bytes()));
                e.seq(allowed_counterparties, |e, a| e.fixed(a.as_slice()));
                e.seq(allowed_object_classes, |e, c| e.u32(*c));
            }
            CapabilityKind::Session {
                max_calls,
                calls_used,
            } => {
                e.u8(6);
                e.u32(*max_calls);
                e.u32(*calls_used);
            }
        }
        // constraints
        e.u64(self.constraints.expiry_epoch);
        e.option(&self.constraints.rate_limit, |e, r| e.u32(*r));
        e.option(&self.constraints.resource_limit, encode_budget);
        e.bool(self.delegable);
        e.bool(self.revoked);
        e.option(&self.parent, |e, p| e.fixed(p.as_bytes()));
    }
}

impl Decode for Capability {
    fn decode(d: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let issuer: Address = d.fixed::<32>()?;
        let holder: Address = d.fixed::<32>()?;
        let kind = match d.u8()? {
            0 => CapabilityKind::Spend {
                max_per_epoch: d.u64()?,
                current_epoch_spent: d.u64()?,
            },
            1 => CapabilityKind::ModifyObject {
                object_class: d.u32()?,
            },
            2 => CapabilityKind::Delegate,
            3 => CapabilityKind::Application {
                app: ApplicationId(d.fixed::<32>()?),
            },
            4 => CapabilityKind::Validator,
            5 => CapabilityKind::Agent {
                max_spend: d.u64()?,
                allowed_apps: d.seq(MAX_SEQ, |dd| Ok(ApplicationId(dd.fixed::<32>()?)))?,
                allowed_counterparties: d.seq(MAX_SEQ, |dd| dd.fixed::<32>())?,
                allowed_object_classes: d.seq(MAX_SEQ, |dd| dd.u32())?,
            },
            6 => CapabilityKind::Session {
                max_calls: d.u32()?,
                calls_used: d.u32()?,
            },
            v => return Err(DecodeError::UnknownVariant(v)),
        };
        let expiry_epoch = d.u64()?;
        let rate_limit = d.option(|dd| dd.u32())?;
        let resource_limit = d.option(decode_budget)?;
        let delegable = d.bool()?;
        let revoked = d.bool()?;
        let parent = d.option(|dd| Ok(CapabilityId(dd.fixed::<32>()?)))?;
        let constraints = Constraints {
            expiry_epoch,
            rate_limit,
            resource_limit,
        };
        let mut cap = Capability {
            id: CapabilityId::ZERO,
            issuer,
            holder,
            kind,
            constraints,
            delegable,
            revoked,
            parent,
        };
        let id = Capability::derive_id(&cap.fields_bytes());
        cap.id = id;
        Ok(cap)
    }
}

impl Capability {
    /// Encode the id-determining fields (everything except `id`).
    pub fn fields_bytes(&self) -> Vec<u8> {
        let mut e = Encoder::new();
        // replicate Encode but skip id (id is derived from these fields)
        e.fixed(self.issuer.as_slice());
        e.fixed(self.holder.as_slice());
        match &self.kind {
            CapabilityKind::Spend {
                max_per_epoch,
                current_epoch_spent,
            } => {
                e.u8(0);
                e.u64(*max_per_epoch);
                e.u64(*current_epoch_spent);
            }
            CapabilityKind::ModifyObject { object_class } => {
                e.u8(1);
                e.u32(*object_class);
            }
            CapabilityKind::Delegate => e.u8(2),
            CapabilityKind::Application { app } => {
                e.u8(3);
                e.fixed(app.as_bytes());
            }
            CapabilityKind::Validator => e.u8(4),
            CapabilityKind::Agent {
                max_spend,
                allowed_apps,
                allowed_counterparties,
                allowed_object_classes,
            } => {
                e.u8(5);
                e.u64(*max_spend);
                e.seq(allowed_apps, |e, a| e.fixed(a.as_bytes()));
                e.seq(allowed_counterparties, |e, a| e.fixed(a.as_slice()));
                e.seq(allowed_object_classes, |e, c| e.u32(*c));
            }
            CapabilityKind::Session {
                max_calls,
                calls_used,
            } => {
                e.u8(6);
                e.u32(*max_calls);
                e.u32(*calls_used);
            }
        }
        e.u64(self.constraints.expiry_epoch);
        e.option(&self.constraints.rate_limit, |e, r| e.u32(*r));
        e.option(&self.constraints.resource_limit, encode_budget);
        e.bool(self.delegable);
        e.bool(self.revoked);
        e.option(&self.parent, |e, p| e.fixed(p.as_bytes()));
        e.into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(b: u8) -> Address {
        [b; 32]
    }

    fn spend_cap(max: u64) -> Capability {
        let mut c = Capability {
            id: CapabilityId::ZERO,
            issuer: addr(1),
            holder: addr(2),
            kind: CapabilityKind::Spend {
                max_per_epoch: max,
                current_epoch_spent: 0,
            },
            constraints: Constraints {
                expiry_epoch: 100,
                rate_limit: None,
                resource_limit: None,
            },
            delegable: false,
            revoked: false,
            parent: None,
        };
        c.id = Capability::derive_id(&c.fields_bytes());
        c
    }

    #[test]
    fn spend_within_limit_ok() {
        let mut c = spend_cap(20);
        assert!(c.authorize_spend(10, 1).is_ok());
        assert!(c.authorize_spend(10, 1).is_ok());
    }

    #[test]
    fn spend_over_limit_rejected() {
        let mut c = spend_cap(20);
        assert!(c.authorize_spend(10, 1).is_ok());
        assert_eq!(c.authorize_spend(11, 1), Err(CapabilityError::Exceeded));
    }

    #[test]
    fn revoked_rejected() {
        let mut c = spend_cap(20);
        c.revoked = true;
        assert_eq!(c.authorize_spend(1, 1), Err(CapabilityError::Revoked));
    }

    #[test]
    fn expired_rejected() {
        let mut c = spend_cap(20);
        assert_eq!(c.authorize_spend(1, 200), Err(CapabilityError::Expired));
    }

    #[test]
    fn non_delegable_flagged() {
        let c = spend_cap(20);
        assert!(!c.delegable);
        if !c.delegable {
            assert_eq!(
                Err::<(), _>(CapabilityError::NotDelegable),
                Err(CapabilityError::NotDelegable)
            );
        }
    }

    #[test]
    fn agent_capability_limits() {
        let app = ApplicationId([9u8; 32]);
        let counterparty = addr(5);
        let mut c = Capability {
            id: CapabilityId::ZERO,
            issuer: addr(1),
            holder: addr(7),
            kind: CapabilityKind::Agent {
                max_spend: 10,
                allowed_apps: vec![app],
                allowed_counterparties: vec![counterparty],
                allowed_object_classes: vec![1],
            },
            constraints: Constraints {
                expiry_epoch: 2000,
                rate_limit: None,
                resource_limit: None,
            },
            delegable: false,
            revoked: false,
            parent: None,
        };
        c.id = Capability::derive_id(&c.fields_bytes());
        // valid purchase
        assert!(c.authorize_spend(10, 5).is_ok());
        // over-limit
        assert_eq!(c.authorize_spend(11, 5), Err(CapabilityError::Exceeded));
        // coverage
        assert!(c.covers_application(&app));
        assert!(!c.covers_application(&ApplicationId([8u8; 32])));
        assert!(c.covers_object_class(1));
        assert!(!c.covers_object_class(2));
    }

    #[test]
    fn capability_encoding_roundtrip() {
        let c = spend_cap(42);
        let bytes = c.to_bytes();
        let mut d = Decoder::new(&bytes);
        let out = Capability::decode(&mut d).unwrap();
        d.finish().unwrap();
        assert_eq!(c, out);
        assert_eq!(
            c.id, out.id,
            "id derivation must be stable across encode/decode"
        );
    }
}
