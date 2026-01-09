//! Versioned envelope for serialized data.
//!
//! Per ADR-006, all serialized data includes a version header/envelope to enable:
//! - Schema evolution
//! - In-place upgrades
//! - Rolling deployments

use serde::{Deserialize, Serialize};

/// The current envelope version.
pub const CURRENT_VERSION: u32 = 1;

/// A versioned envelope that wraps serialized data.
///
/// All data persisted to NATS (events, outputs) or stored in the database
/// should be wrapped in this envelope to support schema evolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope<T> {
    /// The version of the envelope format.
    pub version: u32,
    /// The wrapped payload.
    pub payload: T,
}

impl<T> Envelope<T> {
    /// Creates a new envelope with the current version.
    #[must_use]
    pub fn new(payload: T) -> Self {
        Self {
            version: CURRENT_VERSION,
            payload,
        }
    }

    /// Unwraps the envelope, returning the payload.
    #[must_use]
    pub fn into_payload(self) -> T {
        self.payload
    }

    /// Returns a reference to the payload.
    #[must_use]
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Returns true if this envelope uses the current version.
    #[must_use]
    pub fn is_current_version(&self) -> bool {
        self.version == CURRENT_VERSION
    }
}

impl<T: Serialize> Envelope<T> {
    /// Serializes the envelope to JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

impl<T: for<'de> Deserialize<'de>> Envelope<T> {
    /// Deserializes an envelope from JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// A versioned envelope that supports lazy deserialization of the payload.
///
/// This is useful when you need to check the version before deserializing
/// the full payload, or when migrating between versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawEnvelope {
    /// The version of the envelope format.
    pub version: u32,
    /// The raw payload (not yet deserialized).
    pub payload: serde_json::Value,
}

impl RawEnvelope {
    /// Attempts to deserialize the payload into the given type.
    ///
    /// # Errors
    ///
    /// Returns an error if the payload cannot be deserialized into `T`.
    pub fn deserialize_payload<T: for<'de> Deserialize<'de>>(
        self,
    ) -> Result<Envelope<T>, serde_json::Error> {
        let payload: T = serde_json::from_value(self.payload)?;
        Ok(Envelope {
            version: self.version,
            payload,
        })
    }

    /// Returns the version of this envelope.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Deserializes from JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestPayload {
        message: String,
        count: u32,
    }

    #[test]
    fn envelope_creation() {
        let payload = TestPayload {
            message: "hello".to_string(),
            count: 42,
        };
        let envelope = Envelope::new(payload.clone());

        assert_eq!(envelope.version, CURRENT_VERSION);
        assert_eq!(envelope.payload(), &payload);
        assert!(envelope.is_current_version());
    }

    #[test]
    fn envelope_serde_roundtrip() {
        let payload = TestPayload {
            message: "test".to_string(),
            count: 100,
        };
        let envelope = Envelope::new(payload);

        let bytes = envelope.to_json_bytes().expect("serialize");
        let parsed: Envelope<TestPayload> = Envelope::from_json_bytes(&bytes).expect("deserialize");

        assert_eq!(envelope, parsed);
    }

    #[test]
    fn raw_envelope_lazy_deserialization() {
        let payload = TestPayload {
            message: "lazy".to_string(),
            count: 7,
        };
        let envelope = Envelope::new(payload.clone());
        let bytes = envelope.to_json_bytes().expect("serialize");

        // First deserialize as raw to check version
        let raw: RawEnvelope = RawEnvelope::from_json_bytes(&bytes).expect("deserialize raw");
        assert_eq!(raw.version(), CURRENT_VERSION);

        // Then deserialize the payload
        let typed: Envelope<TestPayload> = raw.deserialize_payload().expect("deserialize payload");
        assert_eq!(typed.payload, payload);
    }

    #[test]
    fn envelope_json_structure() {
        let payload = TestPayload {
            message: "structure".to_string(),
            count: 1,
        };
        let envelope = Envelope::new(payload);
        let json = serde_json::to_value(&envelope).expect("to_value");

        // Verify the structure includes version at the top level
        assert!(json.get("version").is_some());
        assert!(json.get("payload").is_some());
        assert_eq!(json["version"], CURRENT_VERSION);
    }
}
