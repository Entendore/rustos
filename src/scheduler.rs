use alloc::boxed::Box;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;
use x86_64::VirtAddr;

use crate::resource::Resource;
use crate::KernelError;

pub type TaskFn = fn(Vec<String>) -> !;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State { Ready, Running, Blocked, Terminated }

#[repr(C)]
pub struct Stack([u8; 4096 * 8]);

#[repr(C)]
struct Task {
    id: usize,
    name: String,
    state: State,
    rsp: VirtAddr,
    entry: Option<TaskFn>,
    args: Option<Vec<String>>,
    stack: Option<Box<Stack>>,
    block_until: usize,
    exit_code: Option<u64>,
    fds: BTreeMap<usize, Box<dyn Resource>>,
    next_fd: usize,
}

pub struct Scheduler {
    queue: VecDeque<usize>,
    current: Option<usize>,
    tasks: Vec<Task>,
    ticks: AtomicUsize,
}

pub static SCHED: Mutex<Scheduler> = Mutex::new(Scheduler {
    queue: VecDeque::new(),
    current: None,
    tasks: Vec::new(),
    ticks: AtomicUsize::new(0),
});

pub fn init() {
    spawn("idle", Vec::new(), idle_task);
}

extern "C" fn task_trampoline() -> ! {
    let (f, args) = {
        let mut s = SCHED.lock();
        let cur = s.current.unwrap();
        let task = &mut s.tasks[cur];
        (task.entry.take().unwrap(), task.args.take().unwrap_or_default())
    };
    f(args)
}

pub fn spawn(name: &str, args: Vec<String>, entry: TaskFn) -> usize {
    let mut stack = Box::new(Stack([0; 4096 * 8]));
    let top = VirtAddr::from_ptr(stack.0.as_ptr()) + 4096 * 8;
    let mut sp = top.as_u64();
    unsafe {
        sp -= 8; *(sp as *mut u64) = task_trampoline as u64;
        sp -= 8 * 6;
        core::ptr::write_bytes(sp as *mut u64, 0, 6);
    }
    let id = {
        let mut s = SCHED.lock();
        let id = s.tasks.len();
        
        let mut fds = BTreeMap::new();
        fds.insert(0, Box::new(crate::resource::Stdin) as Box<dyn Resource>);
        fds.insert(1, Box::new(crate::resource::Stdout) as Box<dyn Resource>);
        
        s.tasks.push(Task {
            id, name: String::from(name),
            state: State::Ready,
            rsp: VirtAddr::new(sp),
            entry: Some(entry),
            args: Some(args),
            stack: Some(stack),
            block_until: 0,
            exit_code: None,
            fds,
            next_fd: 2,
        });
        s.queue.push_back(id);
        id
    };
    id
}

pub fn open_resource(res: Box<dyn Resource>) -> Result<usize, KernelError> {
    let mut s = SCHED.lock();
    let cur = s.current.ok_or(KernelError::Invalid)?;
    let fd = s.tasks[cur].next_fd;
    s.tasks[cur].next_fd += 1;
    s.tasks[cur].fds.insert(fd, res);
    Ok(fd)
}

pub fn read_resource(fd: usize, buf: &mut [u8]) -> Result<usize, KernelError> {
    let mut s = SCHED.lock();
    let cur = s.current.ok_or(KernelError::Invalid)?;
    let res = s.tasks[cur].fds.get_mut(&fd).ok_or(KernelError::BadFd)?;
    res.read(buf)
}

pub fn write_resource(fd: usize, buf: &[u8]) -> Result<usize, KernelError> {
    let mut s = SCHED.lock();
    let cur = s.current.ok_or(KernelError::Invalid)?;
    let res = s.tasks[cur].fds.get_mut(&fd).ok_or(KernelError::BadFd)?;
    res.write(buf)
}

pub fn tick() {
    let mut s = SCHED.lock();
    s.ticks.fetch_add(1, Ordering::Relaxed);
    s.check_sleeping();
    s.reap_zombies();
}

pub fn yield_now() {
    let next = {
        let mut s = SCHED.lock();
        if s.queue.is_empty() { return; }
        let cur = match s.current {
            Some(c) => c,
            None => { return; }
        };
        let n = s.queue.pop_front().unwrap();
        
        if s.tasks[cur].state == State::Running || s.tasks[cur].state == State::Ready {
            s.tasks[cur].state = State::Ready;
            s.queue.push_back(cur);
        }
        s.tasks[n].state = State::Running;
        s.current = Some(n);
        (s.tasks[cur].rsp, s.tasks[n].rsp)
    };
    unsafe { context_switch(next.0, next.1); }
}

pub fn run() -> ! {
    let first = {
        let mut s = SCHED.lock();
        let n = s.queue.pop_front().expect("no tasks");
        s.tasks[n].state = State::Running;
        s.current = Some(n);
        s.tasks[n].rsp
    };
    unsafe { context_switch(VirtAddr::zero(), first); }
    unreachable!()
}

pub fn exit_current(code: u64) -> ! {
    {
        let mut s = SCHED.lock();
        let cur = s.current.unwrap();
        s.tasks[cur].state = State::Terminated;
        s.tasks[cur].exit_code = Some(code);
        s.tasks[cur].fds.clear();
        crate::debug!("Task {} exited with code {}", cur, code);
    }
    yield_now();
    unreachable!()
}

pub fn kill(pid: usize) -> Result<(), KernelError> {
    let mut s = SCHED.lock();
    if pid == 0 || pid == 1 {
        return Err(KernelError::Permission);
    }
    if let Some(task) = s.tasks.get_mut(pid) {
        task.state = State::Terminated;
        task.exit_code = Some(0xDEAD);
        task.fds.clear();
        crate::info!("Killed task {}", pid);
        Ok(())
    } else {
        Err(KernelError::NotFound)
    }
}

pub fn sleep(seconds: u64) {
    let mut s = SCHED.lock();
    if let Some(cur) = s.current {
        s.tasks[cur].state = State::Blocked;
        s.tasks[cur].block_until = s.ticks.load(Ordering::Relaxed) + (seconds * 5) as usize;
    }
}

pub fn get_ticks() -> usize {
    SCHED.lock().ticks.load(Ordering::Relaxed)
}

pub fn get_current_pid() -> usize {
    SCHED.lock().current.unwrap_or(0)
}

pub fn list() -> Vec<(usize, String, State, Option<u64>)> {
    SCHED.lock().tasks.iter()
        .map(|t| (t.id, t.name.clone(), t.state, t.exit_code))
        .collect()
}

impl Scheduler {
    fn check_sleeping(&mut self) {
        let cur_ticks = self.ticks.load(Ordering::Relaxed);
        let mut unblocked = Vec::new();
        for task in self.tasks.iter_mut() {
            if task.state == State::Blocked && task.block_until > 0 && cur_ticks >= task.block_until {
                task.state = State::Ready;
                task.block_until = 0;
                unblocked.push(task.id);
            }
        }
        for id in unblocked {
            self.queue.push_back(id);
        }
    }

    fn reap_zombies(&mut self) {
        for task in self.tasks.iter_mut() {
            if task.state == State::Terminated {
                if task.stack.take().is_some() {
                    crate::debug!("Reaped zombie task {}", task.id);
                }
            }
        }
    }
}

fn idle_task(_args: Vec<String>) -> ! {
    loop {
        crate::executor::poll_all();
        x86_64::instructions::hlt();
    }
}

#[naked]
unsafe extern "sysv64" fn context_switch(_from: VirtAddr, _to: VirtAddr) {
    core::arch::asm!(
        "push rbp", "push rbx", "push r12", "push r13", "push r14", "push r15",
        "mov [rdi], rsp",
        "mov rsp, [rsi]",
        "pop r15", "pop r14", "pop r13", "pop r12", "pop rbx", "pop rbp",
        "ret",
        options(noreturn)
    );
}
