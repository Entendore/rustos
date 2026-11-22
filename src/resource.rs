use crate::KernelError;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

pub trait Resource: Send {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, KernelError>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, KernelError>;
}

pub struct Stdin;
impl Resource for Stdin {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, KernelError> {
        let mut i = 0;
        while i < buf.len() {
            if let Some(b) = crate::driver::keyboard::try_get() {
                buf[i] = b;
                i += 1;
            } else {
                break;
            }
        }
        Ok(i)
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, KernelError> {
        Err(KernelError::Permission)
    }
}

pub struct Stdout;
impl Resource for Stdout {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, KernelError> {
        Err(KernelError::Permission)
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, KernelError> {
        crate::print!("{}", core::str::from_utf8(buf).unwrap_or(""));
        Ok(buf.len())
    }
}

pub struct File {
    path: String,
    data: Vec<u8>,
    offset: usize,
}

impl File {
    pub fn open(path: &str) -> Result<Self, KernelError> {
        match crate::fs::read(path) {
            Some(data) => Ok(Self {
                path: String::from(path),
                data,
                offset: 0,
            }),
            None => Err(KernelError::NotFound),
        }
    }
}

impl Resource for File {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, KernelError> {
        if self.offset >= self.data.len() {
            return Ok(0);
        }
        let remaining = &self.data[self.offset..];
        let to_read = core::cmp::min(buf.len(), remaining.len());
        buf[..to_read].copy_from_slice(&remaining[..to_read]);
        self.offset += to_read;
        Ok(to_read)
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, KernelError> {
        crate::fs::write(&self.path, buf);
        self.data = buf.to_vec();
        self.offset = 0;
        Ok(buf.len())
    }
}

pub struct PipeReader(Arc<Mutex<VecDeque<u8>>>);
pub struct PipeWriter(Arc<Mutex<VecDeque<u8>>>);

pub fn create_pipe() -> (PipeReader, PipeWriter) {
    let buf = Arc::new(Mutex::new(VecDeque::new()));
    (PipeReader(buf.clone()), PipeWriter(buf))
}

impl Resource for PipeReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, KernelError> {
        loop {
            let mut queue = self.0.lock();
            if queue.is_empty() {
                drop(queue);
                crate::scheduler::yield_now();
            } else {
                let mut i = 0;
                while i < buf.len() {
                    if let Some(b) = queue.pop_front() {
                        buf[i] = b;
                        i += 1;
                    } else {
                        break;
                    }
                }
                return Ok(i);
            }
        }
    }
    fn write(&mut self, _buf: &[u8]) -> Result<usize, KernelError> {
        Err(KernelError::Permission)
    }
}

impl Resource for PipeWriter {
    fn read(&mut self, _buf: &mut [u8]) -> Result<usize, KernelError> {
        Err(KernelError::Permission)
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, KernelError> {
        let mut queue = self.0.lock();
        for &b in buf {
            queue.push_back(b);
        }
        Ok(buf.len())
    }
}
