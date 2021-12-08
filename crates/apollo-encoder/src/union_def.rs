use std::fmt;

use crate::StringValue;

/// UnionDefs are an abstract type where no common fields are declared.
///
/// *UnionDefTypeDefinition*:
///     Description? **union** Name Directives? UnionDefMemberTypes?
///
/// Detailed documentation can be found in [GraphQL spec](https://spec.graphql.org/October2021/#sec-UnionDef).
///
/// ### Example
/// ```rust
/// use apollo_encoder::{UnionDef};
///
/// let mut union_ = UnionDef::new("Pet".to_string());
/// union_.member("Cat".to_string());
/// union_.member("Dog".to_string());
///
/// assert_eq!(
///     union_.to_string(),
/// r#"union Pet = Cat | Dog
/// "#
/// );
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct UnionDef {
    // Name must return a String.
    name: String,
    // Description may return a String.
    description: StringValue,
    // The vector of members that can be represented within this union.
    members: Vec<String>,
}

impl UnionDef {
    /// Create a new instance of a UnionDef.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            description: StringValue::Top { source: None },
            members: Vec::new(),
        }
    }

    /// Set the UnionDefs description.
    pub fn description(&mut self, description: &str) {
        self.description = StringValue::Top {
            source: Some(description.to_owned()),
        };
    }

    /// Set a UnionDef member.
    pub fn member(&mut self, member: &str) {
        self.members.push(member.to_owned());
    }
}

impl fmt::Display for UnionDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)?;

        write!(f, "union {} = ", self.name)?;

        for (i, member) in self.members.iter().enumerate() {
            match i {
                0 => write!(f, "{}", member)?,
                _ => write!(f, " | {}", member)?,
            }
        }

        writeln!(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_encodes_union_with_description() {
        let mut union_ = UnionDef::new("Pet".to_string());
        union_.description(Some("A union of all animals in a household.".to_string()));
        union_.member("Cat".to_string());
        union_.member("Dog".to_string());

        assert_eq!(
            union_.to_string(),
            r#""A union of all animals in a household."
union Pet = Cat | Dog
"#
        );
    }
}
