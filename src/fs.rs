use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::RwLock;

#[derive(Clone)]
pub enum FsNode {
    File(Vec<u8>),
    Directory(BTreeMap<String, FsNode>),
}

static FS: RwLock<BTreeMap<String, FsNode>> = RwLock::new(BTreeMap::new());

pub fn init() {
    let mut fs = FS.write();
    fs.clear();
    fs.insert("/".to_string(), FsNode::Directory(BTreeMap::new()));
    
    mkdir("/etc");
    mkdir("/bin");
    
    write("/etc/motd", b"Welcome to RustOS!");
    write("/etc/version", b"rustos 0.1.0");
}

fn split_path(path: &str) -> Option<(&str, &str)> {
    if path == "/" { return None; }
    let trimmed = path.trim_end_matches('/');
    let idx = trimmed.rfind('/')?;
    let parent = &trimmed[..idx + 1];
    let name = &trimmed[idx + 1..];
    if name.is_empty() { None } else { Some((parent, name)) }
}

pub fn mkdir(path: &str) {
    let mut fs = FS.write();
    if let Some((parent, name)) = split_path(path) {
        if let Some(FsNode::Directory(dir)) = fs.get_mut(parent) {
            dir.insert(name.to_string(), FsNode::Directory(BTreeMap::new()));
        }
    } else if !fs.contains_key(path) {
        fs.insert(path.to_string(), FsNode::Directory(BTreeMap::new()));
    }
}

pub fn list(path: &str) -> Vec<String> {
    let fs = FS.read();
    let path = if path.is_empty() { "/" } else { path };
    
    if let Some(FsNode::Directory(dir)) = fs.get(path) {
        dir.keys().cloned().collect()
    } else {
        Vec::new()
    }
}

pub fn read(path: &str) -> Option<Vec<u8>> {
    let fs = FS.read();
    if let Some(FsNode::File(data)) = fs.get(path) {
        Some(data.clone())
    } else {
        None
    }
}

pub fn write(path: &str, data: &[u8]) {
    let mut fs = FS.write();
    if let Some((parent, name)) = split_path(path) {
        if let Some(FsNode::Directory(dir)) = fs.get_mut(parent) {
            dir.insert(name.to_string(), FsNode::File(data.to_vec()));
        }
    }
}

pub fn delete(path: &str) -> bool {
    let mut fs = FS.write();
    if let Some((parent, name)) = split_path(path) {
        if let Some(FsNode::Directory(dir)) = fs.get_mut(parent) {
            return dir.remove(name).is_some();
        }
    }
    fs.remove(path).is_some()
}
