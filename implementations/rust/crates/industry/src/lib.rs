//! Cross-industry evidence profiles.
//!
//! Industry payloads often contain personal, regulated, or commercially
//! sensitive data. Veridag anchors their canonical bytes and identifiers; it
//! does not require raw business documents to be placed in shared state.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use serde_json::Value;
use thiserror::Error;
use veridag_codec::{Decode, DecodeError, Decoder, Encode, Encoder};
use veridag_crypto::hash;
use veridag_protocol_types::Hash;

/// Supported interoperability profiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndustryProfile {
    /// CNCF CloudEvents 1.0 JSON envelope.
    CloudEvents = 0,
    /// GS1 EPCIS 2.x JSON/JSON-LD document.
    Gs1Epcis = 1,
    /// HL7 FHIR Provenance or AuditEvent resource.
    Hl7Fhir = 2,
    /// OPC UA event projection used by an edge gateway.
    OpcUa = 3,
    /// W3C Verifiable Credential 2.0 document.
    W3cVerifiableCredential = 4,
    /// ISO 20022 message; detailed financial parsing lives in `veridag-stablecoin`.
    Iso20022 = 5,
}

impl IndustryProfile {
    fn from_tag(tag: u8) -> Result<Self, DecodeError> {
        match tag {
            0 => Ok(Self::CloudEvents),
            1 => Ok(Self::Gs1Epcis),
            2 => Ok(Self::Hl7Fhir),
            3 => Ok(Self::OpcUa),
            4 => Ok(Self::W3cVerifiableCredential),
            5 => Ok(Self::Iso20022),
            _ => Err(DecodeError::UnknownVariant(tag)),
        }
    }
}

/// A deterministic, privacy-preserving record committed to Veridag state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceAnchor {
    /// Adapter profile used to validate the source document.
    pub profile: IndustryProfile,
    /// Consortium tenant identifier.
    pub tenant_id: [u8; 32],
    /// Source-system event identifier, hashed before anchoring.
    pub event_id_digest: Hash,
    /// Normative schema/profile URI digest.
    pub schema_digest: Hash,
    /// Source organization/system URI digest.
    pub source_digest: Hash,
    /// Exact source document byte digest.
    pub payload_digest: Hash,
    /// Source-provided event time in Unix milliseconds.
    pub occurred_at_ms: u64,
}

impl EvidenceAnchor {
    /// Build an anchor after applying the selected profile's minimum
    /// interoperability checks.
    pub fn from_document(
        profile: IndustryProfile,
        tenant_id: [u8; 32],
        event_id: &str,
        schema_uri: &str,
        source_uri: &str,
        occurred_at_ms: u64,
        document: &[u8],
    ) -> Result<Self, IndustryError> {
        if event_id.is_empty() || event_id.len() > 512 {
            return Err(IndustryError::InvalidField("event_id"));
        }
        if schema_uri.is_empty() || schema_uri.len() > 1024 {
            return Err(IndustryError::InvalidField("schema_uri"));
        }
        if source_uri.is_empty() || source_uri.len() > 1024 {
            return Err(IndustryError::InvalidField("source_uri"));
        }
        if document.is_empty() || document.len() > 1024 * 1024 {
            return Err(IndustryError::DocumentSize(document.len()));
        }
        validate_document(profile, document)?;
        Ok(Self {
            profile,
            tenant_id,
            event_id_digest: hash("VERIDAG_EVIDENCE_EVENT_ID_V1", event_id.as_bytes()),
            schema_digest: hash("VERIDAG_EVIDENCE_SCHEMA_V1", schema_uri.as_bytes()),
            source_digest: hash("VERIDAG_EVIDENCE_SOURCE_V1", source_uri.as_bytes()),
            payload_digest: hash("VERIDAG_EVIDENCE_PAYLOAD_V1", document),
            occurred_at_ms,
        })
    }

    /// Stable identifier for the anchor.
    pub fn id(&self) -> Hash {
        hash("VERIDAG_EVIDENCE_ANCHOR_V1", &self.to_bytes())
    }
}

impl Encode for EvidenceAnchor {
    fn encode(&self, encoder: &mut Encoder) {
        encoder.u8(self.profile as u8);
        encoder.fixed(&self.tenant_id);
        encoder.fixed(&self.event_id_digest);
        encoder.fixed(&self.schema_digest);
        encoder.fixed(&self.source_digest);
        encoder.fixed(&self.payload_digest);
        encoder.u64(self.occurred_at_ms);
    }
}

impl Decode for EvidenceAnchor {
    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        Ok(Self {
            profile: IndustryProfile::from_tag(decoder.u8()?)?,
            tenant_id: decoder.fixed::<32>()?,
            event_id_digest: decoder.fixed::<32>()?,
            schema_digest: decoder.fixed::<32>()?,
            source_digest: decoder.fixed::<32>()?,
            payload_digest: decoder.fixed::<32>()?,
            occurred_at_ms: decoder.u64()?,
        })
    }
}

/// Industry adapter errors.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IndustryError {
    /// Document size is outside the accepted range.
    #[error("industry document size is invalid: {0}")]
    DocumentSize(usize),
    /// JSON parsing failed.
    #[error("industry document is not valid JSON")]
    InvalidJson,
    /// A required profile field is absent or invalid.
    #[error("industry document has invalid required field: {0}")]
    InvalidField(&'static str),
    /// ISO 20022 is routed to its dedicated XML parser.
    #[error("ISO 20022 documents must use the veridag-stablecoin ISO 20022 adapter")]
    DedicatedIso20022Adapter,
}

/// Validate the minimum interoperable shape of a source document.
///
/// This is an admission check, not a replacement for the normative standard's
/// complete conformance suite. Industry packs can layer stricter validation
/// without changing the consensus-visible anchor encoding.
pub fn validate_document(profile: IndustryProfile, document: &[u8]) -> Result<(), IndustryError> {
    if profile == IndustryProfile::Iso20022 {
        return Err(IndustryError::DedicatedIso20022Adapter);
    }
    let value: Value = serde_json::from_slice(document).map_err(|_| IndustryError::InvalidJson)?;
    let object = value.as_object().ok_or(IndustryError::InvalidJson)?;
    match profile {
        IndustryProfile::CloudEvents => {
            require_string(object, "id")?;
            require_string(object, "source")?;
            require_string(object, "type")?;
            if object.get("specversion").and_then(Value::as_str) != Some("1.0") {
                return Err(IndustryError::InvalidField("specversion"));
            }
        }
        IndustryProfile::Gs1Epcis => {
            if object.get("type").and_then(Value::as_str) != Some("EPCISDocument") {
                return Err(IndustryError::InvalidField("type"));
            }
            if value
                .pointer("/epcisBody/eventList")
                .and_then(Value::as_array)
                .is_none()
            {
                return Err(IndustryError::InvalidField("epcisBody.eventList"));
            }
        }
        IndustryProfile::Hl7Fhir => match object.get("resourceType").and_then(Value::as_str) {
            Some("Provenance" | "AuditEvent") => {}
            _ => return Err(IndustryError::InvalidField("resourceType")),
        },
        IndustryProfile::OpcUa => {
            require_string(object, "nodeId")?;
            require_string(object, "browseName")?;
            require_string(object, "sourceTimestamp")?;
        }
        IndustryProfile::W3cVerifiableCredential => {
            if !contains_string(object.get("type"), "VerifiableCredential") {
                return Err(IndustryError::InvalidField("type"));
            }
            if object.get("issuer").is_none() {
                return Err(IndustryError::InvalidField("issuer"));
            }
            if object.get("credentialSubject").is_none() {
                return Err(IndustryError::InvalidField("credentialSubject"));
            }
        }
        IndustryProfile::Iso20022 => unreachable!(),
    }
    Ok(())
}

fn require_string(
    object: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<(), IndustryError> {
    match object.get(field).and_then(Value::as_str) {
        Some(value) if !value.is_empty() => Ok(()),
        _ => Err(IndustryError::InvalidField(field)),
    }
}

fn contains_string(value: Option<&Value>, expected: &str) -> bool {
    match value {
        Some(Value::String(value)) => value == expected,
        Some(Value::Array(values)) => values.iter().any(|value| value.as_str() == Some(expected)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor(profile: IndustryProfile, document: &[u8]) -> Result<EvidenceAnchor, IndustryError> {
        EvidenceAnchor::from_document(
            profile,
            [7; 32],
            "event-123",
            "https://schemas.example/profile/v1",
            "urn:example:source",
            1_725_000_000_000,
            document,
        )
    }

    #[test]
    fn cloud_event_anchor_is_deterministic_and_roundtrips() {
        let document =
            br#"{"specversion":"1.0","id":"42","source":"urn:test","type":"example.created"}"#;
        let first = anchor(IndustryProfile::CloudEvents, document).unwrap();
        let second = anchor(IndustryProfile::CloudEvents, document).unwrap();
        assert_eq!(first.id(), second.id());
        let bytes = first.to_bytes();
        let mut decoder = Decoder::new(&bytes);
        let decoded = EvidenceAnchor::decode(&mut decoder).unwrap();
        decoder.finish().unwrap();
        assert_eq!(decoded, first);
    }

    #[test]
    fn profile_examples_are_admitted() {
        anchor(
            IndustryProfile::Gs1Epcis,
            br#"{"type":"EPCISDocument","epcisBody":{"eventList":[]}}"#,
        )
        .unwrap();
        anchor(
            IndustryProfile::Hl7Fhir,
            br#"{"resourceType":"Provenance","target":[]}"#,
        )
        .unwrap();
        anchor(
            IndustryProfile::OpcUa,
            br#"{"nodeId":"ns=2;i=1","browseName":"Temperature","sourceTimestamp":"2026-01-01T00:00:00Z"}"#,
        )
        .unwrap();
        anchor(
            IndustryProfile::W3cVerifiableCredential,
            br#"{"@context":["https://www.w3.org/ns/credentials/v2"],"type":["VerifiableCredential"],"issuer":"did:web:example.com","credentialSubject":{"id":"did:example:123"}}"#,
        )
        .unwrap();
    }

    #[test]
    fn malformed_profile_is_rejected() {
        assert_eq!(
            anchor(
                IndustryProfile::CloudEvents,
                br#"{"specversion":"0.3","id":"42"}"#,
            ),
            Err(IndustryError::InvalidField("source"))
        );
    }

    #[test]
    fn raw_payload_is_not_present_in_anchor_bytes() {
        let secret = b"secret-patient-value";
        let document = br#"{"resourceType":"AuditEvent","text":"secret-patient-value"}"#;
        let encoded = anchor(IndustryProfile::Hl7Fhir, document)
            .unwrap()
            .to_bytes();
        assert!(!encoded.windows(secret.len()).any(|window| window == secret));
    }
}
