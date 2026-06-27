use std::collections::HashMap;

use crate::gen::generator::Generator;

/// Holds the set of available [`Generator`] implementations keyed by name.
pub struct GeneratorRegistry {
    inner: HashMap<String, Box<dyn Generator>>,
}

impl Default for GeneratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl GeneratorRegistry {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Register a generator; overwrites any prior registration under the same name.
    pub fn register(&mut self, g: Box<dyn Generator>) {
        self.inner.insert(g.name().to_owned(), g);
    }

    /// Look up a generator by its machine-readable name.
    pub fn get(&self, name: &str) -> Option<&dyn Generator> {
        self.inner.get(name).map(|b| b.as_ref())
    }

    /// Enumerate registered generators as `(name, description)` pairs.
    pub fn list(&self) -> Vec<(&str, &str)> {
        let mut pairs: Vec<(&str, &str)> = self
            .inner
            .values()
            .map(|g| (g.name(), g.description()))
            .collect();
        pairs.sort_by_key(|(n, _)| *n);
        pairs
    }
}
