use crate::Error;

/// Represents a replacement where `key` should be replaced with `value`
#[derive(Debug, PartialEq)]
pub struct Replacement {
    pub key: String,
    pub value: String,
}

impl Replacement {
    /// Creates a new replacement.
    ///
    /// `key` will be wrapped in {curly brackets}.
    pub fn new(key: String, value: String) -> Self {
        Self {
            key: format!("{{{}}}", key),
            value,
        }
    }
}

/// A collection of [`crate::replace::Replacement`]
pub struct Replacements {
    replacements: Vec<Replacement>,
}

impl Replacements {
    /// Attempts to parse a string like `key1=value1,key2=value2`
    /// into a collection of [`crate::replace::Replacement`]
    pub fn try_parse_str(to_parse: &str) -> Result<Self, Error> {
        let mut replacements = Vec::new();

        for pair in to_parse.split(",") {
            let mut pair_split = pair.split("=");
            let key = pair_split.next().ok_or(Error::ReplacementParse(
                "replacement does not have key".into(),
            ))?;
            let value = pair_split.next().ok_or(Error::ReplacementParse(format!(
                "replacement '{}' does not have value",
                key
            )))?;

            replacements.push(Replacement::new(key.into(), value.into()));
        }

        Ok(Self { replacements })
    }

    /// Replaces all occurrances of a replacement in the provided string.
    pub fn replace(&self, input: &str) -> String {
        let mut replaced = input.to_string();
        for replacement in &self.replacements {
            replaced = replaced.replace(&replacement.key, &replacement.value);
        }
        replaced
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TO_PARSE_REPLACEMENT_STR: &str = "api_scheme=http,api_host=127.0.0.1:3000";

    #[test]
    fn replacements_try_parse_str() {
        let expected_replacements = vec![
            Replacement {
                key: "{api_scheme}".into(),
                value: "http".into(),
            },
            Replacement {
                key: "{api_host}".into(),
                value: "127.0.0.1:3000".into(),
            },
        ];

        let replacements = Replacements::try_parse_str(TO_PARSE_REPLACEMENT_STR).unwrap();
        assert_eq!(replacements.replacements, expected_replacements);
    }

    #[test]
    fn replacements_try_parse_str_err() {
        assert!(Replacements::try_parse_str("api_scheme,api_host=127.0.0.1:3000,").is_err())
    }

    #[test]
    fn replacements_replace() {
        let replacements = Replacements::try_parse_str(TO_PARSE_REPLACEMENT_STR).unwrap();
        let to_replace = "
        hello
        you are sending requests to {api_scheme}://{api_host}!";
        let expected = "
        hello
        you are sending requests to http://127.0.0.1:3000!";

        assert_eq!(replacements.replace(to_replace), expected)
    }
}
