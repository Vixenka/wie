use std::env;

pub fn is_active(name: &str) -> bool {
    match env::var(name) {
        Ok(v) => matches!(v.as_str(), "1" | "true"),
        Err(_) => false,
    }
}
