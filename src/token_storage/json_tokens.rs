use std::collections::hash_map::Values;
use std::path::Path;
use std::io;
use std::collections::{ HashMap };
use serde::{Deserialize, Serialize};
use yup_oauth2::storage::{
    TokenInfo,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum FilterResponse {
    Maybe,
    No,
}

/// ScopeFilter represents a filter for a set of scopes. It can definitively
/// prove that a given list of scopes is not a subset of another.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ScopeFilter(u64);

impl ScopeFilter {
    /// Determine if this ScopeFilter could be a subset of the provided filter.
    fn is_subset_of(self, filter: ScopeFilter) -> FilterResponse {
        if self.0 & filter.0 == self.0 {
            FilterResponse::Maybe
        } else {
            FilterResponse::No
        }
    }
}

/// ScopeHash is a hash value derived from a list of scopes. The hash value
/// represents a fingerprint of the set of scopes *independent* of the ordering.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ScopeHash(pub u64);

/// A set of scopes
#[derive(Debug)]
pub struct ScopeSet<'a, T> {
    hash: ScopeHash,
    filter: ScopeFilter,
    scopes: &'a [T],
}

// Implement Clone manually. Auto derive fails to work correctly because we want
// Clone to be implemented regardless of whether T is Clone or not.
impl<'a, T> Clone for ScopeSet<'a, T> {
    fn clone(&self) -> Self {
        ScopeSet {
            hash: self.hash,
            filter: self.filter,
            scopes: self.scopes,
        }
    }
}
impl<'a, T> Copy for ScopeSet<'a, T> {}

impl<'a, T> ScopeSet<'a, T>
where
    T: AsRef<str>,
{
    /// Convert from an array into a ScopeSet. Automatically invoked by the compiler when
    /// an array reference is passed.
    // implement an inherent from method even though From is implemented. This
    // is because passing an array ref like &[&str; 1] (&["foo"]) will be auto
    // deref'd to a slice on function boundaries, but it will not implement the
    // From trait. This inherent method just serves to auto deref from array
    // refs to slices and proxy to the From impl.
    pub fn from(scopes: &'a [T]) -> Self {
        let (hash, filter) = scopes.iter().fold(
            (ScopeHash(0), ScopeFilter(0)),
            |(mut scope_hash, mut scope_filter), scope| {
                let h = seahash::hash(scope.as_ref().as_bytes());

                // Use the first 4 6-bit chunks of the seahash as the 4 hash values
                // in the bloom filter.
                for i in 0..4 {
                    // h is a hash derived value in the range 0..64
                    let h = (h >> (6 * i)) & 0b11_1111;
                    scope_filter.0 |= 1 << h;
                }

                // xor the hashes together to get an order independent fingerprint.
                scope_hash.0 ^= h;
                (scope_hash, scope_filter)
            },
        );
        ScopeSet {
            hash,
            filter,
            scopes,
        }
    }
}

/// A single stored token.
#[derive(Debug, Clone)]
pub struct JSONToken {
    pub scopes: Vec<String>,
    pub token: TokenInfo,
    pub hash: ScopeHash,
    pub filter: ScopeFilter,
}

impl Serialize for JSONToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct RawJSONToken<'a> {
            scopes: &'a [String],
            token: &'a TokenInfo,
        }
        RawJSONToken {
            scopes: &self.scopes,
            token: &self.token,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JSONToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawJSONToken {
            scopes: Vec<String>,
            token: TokenInfo,
        }
        let RawJSONToken { scopes, token } = RawJSONToken::deserialize(deserializer)?;
        let ScopeSet { hash, filter, .. } = ScopeSet::from(&scopes);
        Ok(JSONToken {
            scopes,
            token,
            hash,
            filter,
        })
    }
}

pub type JSONTokensMap = HashMap<ScopeHash, JSONToken>;

#[derive(Debug, Clone)]
pub struct JSONTokens {
    token_map: JSONTokensMap,
}

impl Default for JSONTokens {
    fn default() -> JSONTokens {
        JSONTokens {
            token_map: HashMap::new(),
        }
    }
}

impl Serialize for JSONTokens {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.token_map.values())
    }
}

impl<'de> Deserialize<'de> for JSONTokens {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;
        impl<'de> serde::de::Visitor<'de> for V {
            type Value = JSONTokens;

            // Format a message stating what data this Visitor expects to receive.
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of JSONToken's")
            }

            fn visit_seq<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: serde::de::SeqAccess<'de>,
            {
                let mut token_map = HashMap::with_capacity(access.size_hint().unwrap_or(0));
                while let Some(json_token) = access.next_element::<JSONToken>()? {
                    token_map.insert(json_token.hash, json_token);
                }
                Ok(JSONTokens { token_map, ..JSONTokens::default() })
            }
        }

        // Instantiate our Visitor and ask the Deserializer to drive
        // it over the input data.
        deserializer.deserialize_seq(V)
    }
}

impl JSONTokens {
    pub fn new() -> Self {
        JSONTokens::default()
    }

    pub async fn load_from_file(filename: &Path) -> Result<Self, io::Error> {
        let contents = tokio::fs::read(filename).await?;
        serde_json::from_slice(&contents).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn iter<'a>(&'a self) -> JSONTokenIterator<'a> {
        JSONTokenIterator::new(&self.token_map)
    }

    pub fn get<T>(
        &self,
        ScopeSet {
            hash,
            filter,
            scopes,
        }: ScopeSet<T>,
    ) -> Option<TokenInfo>
    where
        T: AsRef<str>,
    {
        if let Some(json_token) = self.token_map.get(&hash) {
            return Some(json_token.token.clone());
        }

        let requested_scopes_are_subset_of = |other_scopes: &[String]| {
            scopes
                .iter()
                .all(|s| other_scopes.iter().any(|t| t.as_str() == s.as_ref()))
        };
        // No exact match for the scopes provided. Search for any tokens that
        // exist for a superset of the scopes requested.
        self.token_map
            .values()
            .filter(|json_token| filter.is_subset_of(json_token.filter) == FilterResponse::Maybe)
            .find(|v: &&JSONToken| requested_scopes_are_subset_of(&v.scopes))
            .map(|t: &JSONToken| t.token.clone())
    }

    pub fn set<T>(
        &mut self,
        ScopeSet {
            hash,
            filter,
            scopes,
        }: ScopeSet<T>,
        token: TokenInfo,
    ) -> Result<(), io::Error>
    where
        T: AsRef<str>,
    {
        use std::collections::hash_map::Entry;
        match self.token_map.entry(hash) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().token = token;
            }
            Entry::Vacant(entry) => {
                let json_token = JSONToken {
                    scopes: scopes.iter().map(|x| x.as_ref().to_owned()).collect(),
                    token,
                    hash,
                    filter,
                };
                entry.insert(json_token.clone());
            }
        }
        Ok(())
    }
}

pub struct JSONTokenIterator<'a> {
    data: &'a JSONTokensMap,
    values: Values<'a, ScopeHash, JSONToken>,
    iter: u32,
}

impl<'a> JSONTokenIterator<'a> {
    fn new(json_tokens: &'a JSONTokensMap) -> JSONTokenIterator<'a> {
        JSONTokenIterator {
            data: json_tokens,
            values: json_tokens.values(),
            iter: 0,
        }
    }
}

impl<'a> Iterator for JSONTokenIterator<'a> {
    type Item = &'a JSONToken;

    fn next(&mut self) -> Option<Self::Item> {
        match self.values.next() {
            Some(json_token) => {
                Some(json_token)
            },
            None => None
        }
    }
}