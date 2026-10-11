use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use spin::RwLock;

pub static ENV: RwLock<BTreeMap<String, String>> = RwLock::new(BTreeMap::new());

pub fn init() {
    let mut env = ENV.write();
    env.clear();
    env.insert("PATH".to_string(), "/bin:/sbin".to_string());
    env.insert("USER".to_string(), "root".to_string());
    env.insert("OS".to_string(), "RustOS".to_string());
}

pub fn get(key: &str) -> Option<String> {
    ENV.read().get(key).cloned()
}

pub fn set(key: &str, val: &str) {
    ENV.write().insert(key.to_string(), val.to_string());
}
