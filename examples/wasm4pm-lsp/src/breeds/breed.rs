use serde_json::Value;

pub struct BreedInput {
    pub payload: Value,
}

impl BreedInput {
    pub fn new(payload: Value) -> Self {
        Self { payload }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.payload.get(key)
    }
}

pub trait CognitiveBreed: Send + Sync {
    fn breed_id(&self) -> &'static str;
    fn run(&self, input: &BreedInput) -> Option<Value>;
}
