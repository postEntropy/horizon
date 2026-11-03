//! Read block state strings received from world queries and block events.
//!
//! ```
//! use horizon_mod_sdk::block_state::BlockState;
//!
//! let state = BlockState::parse("example:machine[facing=north,lit=true]").unwrap();
//! assert_eq!(state.id(), "example:machine");
//! assert_eq!(state.property("facing"), Some("north"));
//! ```

/// A borrowed block identifier and its properties.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockState<'a> {
    id: &'a str,
    properties: Vec<(&'a str, &'a str)>,
}

impl<'a> BlockState<'a> {
    /// Parses `namespace:block` or `namespace:block[property=value,...]`.
    /// Malformed properties and duplicate property names are rejected.
    pub fn parse(input: &'a str) -> Option<Self> {
        let (id, properties) = match input.split_once('[') {
            Some((id, rest)) => (id, Some(rest.strip_suffix(']')?)),
            None => (input, None),
        };
        if id.is_empty()
            || id.contains(|c: char| c.is_whitespace() || matches!(c, '[' | ']' | ',' | '='))
        {
            return None;
        }
        let mut parsed = Vec::new();
        if let Some(properties) = properties {
            if properties.is_empty() {
                return None;
            }
            for property in properties.split(',') {
                let (name, value) = property.split_once('=')?;
                let invalid = |s: &str| {
                    s.is_empty()
                        || s.contains(|c: char| {
                            c.is_whitespace() || matches!(c, '[' | ']' | ',' | '=')
                        })
                };
                if invalid(name)
                    || invalid(value)
                    || parsed.iter().any(|(existing, _)| *existing == name)
                {
                    return None;
                }
                parsed.push((name, value));
            }
        }
        Some(Self {
            id,
            properties: parsed,
        })
    }

    /// Returns the block identifier, without properties.
    pub fn id(&self) -> &'a str {
        self.id
    }

    /// Returns a property value, or `None` when the property is absent.
    pub fn property(&self, name: &str) -> Option<&'a str> {
        self.properties
            .iter()
            .find_map(|(key, value)| (*key == name).then_some(*value))
    }

    /// Iterates over properties in their original order.
    pub fn properties(&self) -> impl Iterator<Item = (&'a str, &'a str)> + '_ {
        self.properties.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::BlockState;

    #[test]
    fn reads_states_with_and_without_properties() {
        let plain = BlockState::parse("minecraft:stone").unwrap();
        assert_eq!(plain.id(), "minecraft:stone");
        assert_eq!(plain.property("lit"), None);
        let state = BlockState::parse("example:machine[facing=north,lit=true]").unwrap();
        assert_eq!(state.property("facing"), Some("north"));
        assert_eq!(
            state.properties().collect::<Vec<_>>(),
            vec![("facing", "north"), ("lit", "true")]
        );
    }

    #[test]
    fn rejects_malformed_states() {
        for input in [
            "",
            "a[",
            "a[]",
            "a[x]",
            "a[x=]",
            "a[=x]",
            "a[x=y,]",
            "a[x=y,x=z]",
            "a[x=y]tail",
            "a[x=y=z]",
            "a]",
            "a[x y=z]",
        ] {
            assert!(BlockState::parse(input).is_none(), "{input}");
        }
    }
}
