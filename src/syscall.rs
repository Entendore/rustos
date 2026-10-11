use crate::KernelError;
use alloc::string::String;
use alloc::vec::Vec;

pub const SYS_WRITE: u64 = 0;
pub const SYS_READ: u64 = 1;
pub const SYS_YIELD: u64 = 2;
pub const SYS_EXIT: u64 = 3;
pub const SYS_OPEN: u64 = 4;
pub const SYS_PS: u64 = 5;
pub const SYS_LIST_FILES: u64 = 6;
pub const SYS_LOG: u64 = 7;
pub const SYS_GETENV: u64 = 8;
pub const SYS_SETENV: u64 = 9;
pub const SYS_KILL: u64 = 10;
pub const SYS_PIPE: u64 = 11;
pub const SYS_GETPID: u64 = 12;

pub fn init() {
    crate::info!("Syscall ABI initialized (int 0x80)");
}

pub fn dispatch(num: u64, arg1: u64, arg2: u64, arg3: u64, _arg4: u64, _arg5: u64, _arg6: u64) -> u64 {
    let res = match num {
        SYS_WRITE => sys_write(arg1, arg2 as *const u8, arg3 as usize),
        SYS_READ => sys_read(arg1, arg2 as *mut u8, arg3 as usize),
        SYS_YIELD => { crate::scheduler::yield_now(); Ok(0) },
        SYS_EXIT => { crate::scheduler::exit_current(arg1); Ok(0) },
        SYS_OPEN => sys_open(arg1 as *const u8, arg2 as usize),
        SYS_PS => sys_ps(),
        SYS_LIST_FILES => sys_list_files(),
        SYS_LOG => sys_log(arg1 as *const u8, arg2 as usize),
        SYS_GETENV => sys_getenv(arg1 as *const u8, arg2 as usize, arg3 as *mut u8),
        SYS_SETENV => sys_setenv(arg1 as *const u8, arg2 as usize, arg3 as *const u8, _arg4 as usize),
        SYS_KILL => sys_kill(arg1 as usize),
        SYS_PIPE => sys_pipe(arg1 as *mut usize, arg2 as *mut usize),
        SYS_GETPID => Ok(crate::scheduler::get_current_pid()),
        _ => Err(KernelError::NotFound),
    };

    match res {
        Ok(val) => val as u64,
        Err(e) => -(e as i64) as u64,
    }
}

fn sys_write(fd: u64, buf: *const u8, len: usize) -> Result<usize, KernelError> {
    let slice = unsafe { core::slice::from_raw_parts(buf, len) };
    crate::scheduler::write_resource(fd as usize, slice)
}

fn sys_read(fd: u64, buf: *mut u8, len: usize) -> Result<usize, KernelError> {
    let slice = unsafe { core::slice::from_raw_parts_mut(buf, len) };
    crate::scheduler::read_resource(fd as usize, slice)
}

fn sys_open(path_ptr: *const u8, path_len: usize) -> Result<usize, KernelError> {
    let slice = unsafe { core::slice::from_raw_parts(path_ptr, path_len) };
    let path = core::str::from_utf8(slice).map_err(|_| KernelError::Invalid)?;
    let file = crate::resource::File::open(path)?;
    crate::scheduler::open_resource(alloc::boxed::Box::new(file))
}

fn sys_ps() -> Result<usize, KernelError> {
    for (id, name, state, exit_code) in crate::scheduler::list() {
        crate::println!("  [{id}] {name:?} — {state:?} exit={:?}", exit_code);
    }
    Ok(0)
}

fn sys_list_files() -> Result<usize, KernelError> {
    for name in crate::fs::list("/") {
        crate::println!("  {name}");
    }
    Ok(0)
}

fn sys_log(msg_ptr: *const u8, msg_len: usize) -> Result<usize, KernelError> {
    let slice = unsafe { core::slice::from_raw_parts(msg_ptr, msg_len) };
    let s = core::str::from_utf8(slice).map_err(|_| KernelError::Invalid)?;
    crate::println!("[user] {s}");
    Ok(s.len())
}

fn sys_getenv(key_ptr: *const u8, key_len: usize, val_ptr: *mut u8) -> Result<usize, KernelError> {
    let key_slice = unsafe { core::slice::from_raw_parts(key_ptr, key_len) };
    let key = core::str::from_utf8(key_slice).map_err(|_| KernelError::Invalid)?;
    
    if let Some(val) = crate::env::get(key) {
        let val_bytes = val.as_bytes();
        let len = val_bytes.len();
        unsafe {
            core::ptr::copy_nonoverlapping(val_bytes.as_ptr(), val_ptr, len);
        }
        Ok(len)
    } else {
        Err(KernelError::NotFound)
    }
}

fn sys_setenv(key_ptr: *const u8, key_len: usize, val_ptr: *const u8, val_len: usize) -> Result<usize, KernelError> {
    let key_slice = unsafe { core::slice::from_raw_parts(key_ptr, key_len) };
    let val_slice = unsafe { core::slice::from_raw_parts(val_ptr, val_len) };
    
    let key = core::str::from_utf8(key_slice).map_err(|_| KernelError::Invalid)?;
    let val = core::str::from_utf8(val_slice).map_err(|_| KernelError::Invalid)?;
    
    crate::env::set(key, val);
    Ok(0)
}

fn sys_kill(pid: usize) -> Result<usize, KernelError> {
    crate::scheduler::kill(pid)?;
    Ok(0)
}

fn sys_pipe(read_ptr: *mut usize, write_ptr: *mut usize) -> Result<usize, KernelError> {
    let (reader, writer) = crate::resource::create_pipe();
    let read_fd = crate::scheduler::open_resource(alloc::boxed::Box::new(reader))?;
    let write_fd = crate::scheduler::open_resource(alloc::boxed::Box::new(writer))?;
    
    unsafe {
        *read_ptr = read_fd;
        *write_ptr = write_fd;
    }
    Ok(0)
}
