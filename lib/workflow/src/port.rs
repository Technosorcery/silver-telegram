//! Port system for workflow nodes.
//!
//! Ports are named connection points on nodes. Each port has a JSON Schema
//! that defines the data type it accepts (input) or produces (output).
//!
//! Connections between ports are valid if their schemas are compatible.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// A JSON Schema defining the data type for a port.
///
/// This is a simplified schema representation. In practice, this wraps
/// a full JSON Schema object for validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortSchema {
    /// The JSON Schema definition.
    #[serde(flatten)]
    pub schema: JsonValue,
}

impl PortSchema {
    /// Creates a schema that accepts any value.
    #[must_use]
    pub fn any() -> Self {
        Self {
            schema: serde_json::json!({}),
        }
    }

    /// Creates a schema for a string type.
    #[must_use]
    pub fn string() -> Self {
        Self {
            schema: serde_json::json!({ "type": "string" }),
        }
    }

    /// Creates a schema for a number type.
    #[must_use]
    pub fn number() -> Self {
        Self {
            schema: serde_json::json!({ "type": "number" }),
        }
    }

    /// Creates a schema for a boolean type.
    #[must_use]
    pub fn boolean() -> Self {
        Self {
            schema: serde_json::json!({ "type": "boolean" }),
        }
    }

    /// Creates a schema for an object type.
    #[must_use]
    pub fn object() -> Self {
        Self {
            schema: serde_json::json!({ "type": "object" }),
        }
    }

    /// Creates a schema for an array type.
    #[must_use]
    pub fn array() -> Self {
        Self {
            schema: serde_json::json!({ "type": "array" }),
        }
    }

    /// Creates a schema from a raw JSON value.
    #[must_use]
    pub fn from_json(schema: JsonValue) -> Self {
        Self { schema }
    }

    /// Checks if this schema is compatible with another schema.
    ///
    /// For now, this is a simplified check. A full implementation would
    /// perform proper JSON Schema compatibility analysis.
    #[must_use]
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        // Empty schema (any) is compatible with everything
        if self.schema == serde_json::json!({}) || other.schema == serde_json::json!({}) {
            return true;
        }

        // Simple type equality check for basic types
        if let (Some(self_type), Some(other_type)) =
            (self.schema.get("type"), other.schema.get("type"))
        {
            return self_type == other_type;
        }

        // For complex schemas, assume compatible for now
        // A full implementation would use a proper JSON Schema validator
        true
    }
}

impl Default for PortSchema {
    fn default() -> Self {
        Self::any()
    }
}

/// An input port on a workflow node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputPort {
    /// The name of this port.
    pub name: String,
    /// The JSON Schema defining accepted data types.
    pub schema: PortSchema,
    /// Whether this input is required (must have an incoming edge).
    pub required: bool,
}

impl InputPort {
    /// Creates a new required input port.
    #[must_use]
    pub fn required(name: impl Into<String>, schema: PortSchema) -> Self {
        Self {
            name: name.into(),
            schema,
            required: true,
        }
    }

    /// Creates a new optional input port.
    #[must_use]
    pub fn optional(name: impl Into<String>, schema: PortSchema) -> Self {
        Self {
            name: name.into(),
            schema,
            required: false,
        }
    }
}

/// An output port on a workflow node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputPort {
    /// The name of this port.
    pub name: String,
    /// The JSON Schema defining the produced data type.
    pub schema: PortSchema,
}

impl OutputPort {
    /// Creates a new output port.
    #[must_use]
    pub fn new(name: impl Into<String>, schema: PortSchema) -> Self {
        Self {
            name: name.into(),
            schema,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_schema_compatible_with_all() {
        let any = PortSchema::any();
        let string = PortSchema::string();
        let number = PortSchema::number();

        assert!(any.is_compatible_with(&string));
        assert!(any.is_compatible_with(&number));
        assert!(string.is_compatible_with(&any));
    }

    #[test]
    fn same_type_compatible() {
        let string1 = PortSchema::string();
        let string2 = PortSchema::string();

        assert!(string1.is_compatible_with(&string2));
    }

    #[test]
    fn different_types_not_compatible() {
        let string = PortSchema::string();
        let number = PortSchema::number();

        assert!(!string.is_compatible_with(&number));
    }

    #[test]
    fn input_port_required() {
        let port = InputPort::required("data", PortSchema::string());
        assert!(port.required);
        assert_eq!(port.name, "data");
    }

    #[test]
    fn input_port_optional() {
        let port = InputPort::optional("config", PortSchema::object());
        assert!(!port.required);
        assert_eq!(port.name, "config");
    }

    #[test]
    fn schema_serde_roundtrip() {
        let schema = PortSchema::string();
        let json = serde_json::to_string(&schema).expect("serialize");
        let parsed: PortSchema = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(schema, parsed);
    }
}
