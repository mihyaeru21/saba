use alloc::string::String;

#[cfg(test)]
use alloc::string::ToString;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Attribute {
    name: String,
    value: String,
}

impl Attribute {
    pub fn add_name_char(&mut self, c: char) {
        self.name.push(c);
    }

    pub fn add_value_char(&mut self, c: char) {
        self.value.push(c);
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }

    #[cfg(test)]
    pub fn nv(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
        }
    }
}
