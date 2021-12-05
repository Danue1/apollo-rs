use std::fmt;

use crate::StringValue;

/// The __EnumValue type represents one of possible values of an enum.
///
/// *EnumValueDefinition*:
///     Description? EnumValue Directives?
///
/// Detailed documentation can be found in [GraphQL spec](https://spec.graphql.org/October2021/#sec-The-__EnumValue-Type).
#[derive(Debug, PartialEq, Clone)]
pub struct EnumValue {
    // Name must return a String.
    name: String,
    // Description may return a String or null.
    description: StringValue,
    // Deprecated returns true if this enum value should no longer be used, otherwise false.
    is_deprecated: bool,
    // Deprecation reason optionally provides a reason why this enum value is deprecated.
    deprecation_reason: StringValue,
}

/// ### Example
/// ```rust
/// use apollo_encoder::{EnumValueBuilder};
///
/// let enum_value = EnumValueBuilder::new("CARDBOARD_BOX")
///     .description("Box nap spot.")
///     .deprecated("Box was recycled.")
///     .build();
///
/// assert_eq!(
///     enum_value.to_string(),
///     r#"  "Box nap spot."
///   CARDBOARD_BOX @deprecated(reason: "Box was recycled.")"#
/// );
/// ```
#[derive(Debug, Clone)]
pub struct EnumValueBuilder {
    // Name must return a String.
    name: String,
    // Description may return a String or null.
    description: Option<String>,
    // Deprecation reason optionally provides a reason why this enum value is deprecated.
    deprecation_reason: Option<String>,
}

impl EnumValueBuilder {
    /// Create a new instance of EnumValueBuilder.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
            deprecation_reason: None,
        }
    }

    /// Set the Enum Value's description.
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set the Enum Value's deprecation properties.
    pub fn deprecated(mut self, reason: &str) -> Self {
        self.deprecation_reason = Some(reason.to_string());
        self
    }

    /// Create a new instance of EnumValue.
    pub fn build(self) -> EnumValue {
        EnumValue {
            name: self.name,
            description: StringValue::Field {
                source: self.description,
            },
            is_deprecated: self.deprecation_reason.is_some(),
            deprecation_reason: StringValue::Reason {
                source: self.deprecation_reason,
            },
        }
    }
}

impl fmt::Display for EnumValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)?;
        write!(f, "  {}", self.name)?;

        if self.is_deprecated {
            write!(f, " @deprecated")?;
            if let StringValue::Reason { source: _ } = &self.deprecation_reason {
                write!(f, "(reason:")?;
                write!(f, "{}", self.deprecation_reason)?;
                write!(f, ")")?
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::EnumValueBuilder;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_encodes_an_enum_value() {
        let enum_value = EnumValueBuilder::new("CAT_TREE").build();

        assert_eq!(enum_value.to_string(), "  CAT_TREE");
    }

    #[test]
    fn it_encodes_an_enum_value_with_desciption() {
        let enum_value = EnumValueBuilder::new("CAT_TREE")
            .description("Top bunk of a cat tree.")
            .build();

        assert_eq!(
            enum_value.to_string(),
            r#"  "Top bunk of a cat tree."
  CAT_TREE"#
        );
    }
    #[test]
    fn it_encodes_an_enum_value_with_deprecated() {
        let enum_value = EnumValueBuilder::new("CARDBOARD_BOX")
            .description("Box nap\nspot.")
            .deprecated("Box was recycled.")
            .build();

        assert_eq!(
            enum_value.to_string(),
            r#"  """
  Box nap
  spot.
  """
  CARDBOARD_BOX @deprecated(reason: "Box was recycled.")"#
        );
    }

    #[test]
    fn it_encodes_an_enum_value_with_deprecated_block_string_value() {
        let enum_value = EnumValueBuilder::new("CARDBOARD_BOX")
            .description("Box nap\nspot.")
            .deprecated("Box was \"recycled\".")
            .build();

        assert_eq!(
            enum_value.to_string(),
            r#"  """
  Box nap
  spot.
  """
  CARDBOARD_BOX @deprecated(reason:
  """
  Box was "recycled".
  """
  )"#
        );
    }
}
