use alloc::string::String;
use alloc::vec::Vec;
use crate::scheduler;

const PROMPT: &str = "rustos> ";

pub fn spawn() {
    let args = Vec::new();
    scheduler::spawn("shell", args, main);
    crate::executor::spawn(heartbeat());
}

async fn heartbeat() {
    let mut ticks = 0;
    loop {
        crate::scheduler::yield_now();
        ticks += 1;
        if ticks % 500 == 0 {
            crate::info!("[async] heartbeat tick {}", ticks);
        }
    }
}

fn main(_args: Vec<String>) -> ! {
    crate::println!("\n=== RustOS Shell ===");
    crate::println!("Commands: help, ls <path>, cat <path>, write <path> <text>, mkdir <path>, ps, kill <pid>, run <path>, yield, sleep <s>, log <lvl>, mem, getenv, setenv, cpuinfo, getpid, uptime, clear, reboot, shutdown\n");

    let mut history: Vec<String> = Vec::new();
    let mut line = String::new();
    
    loop {
        crate::print!("{PROMPT}");
        line.clear();
        read_line(&mut line, &mut history);
        crate::println!();
        if line.trim().is_empty() { continue; }
        
        history.push(line.clone());
        if history.len() > 10 { history.remove(0); }

        execute_command(&line);
    }
}

fn execute_command(raw: &str) {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.is_empty() { return; }
    let cmd = parts[0];
    let args = &parts[1..];
    match cmd {
        "help"  => print_help(),
        "ls"    => ls_cmd(args),
        "cat"   => cat(args),
        "write" => write_cmd(args),
        "mkdir" => mkdir_cmd(args),
        "rm"    => rm(args),
        "ps"    => { for (id, name, state, exit_code) in crate::scheduler::list() { crate::println!("  [{id}] {name:?} — {state:?} exit={:?}", exit_code); } }
        "kill"  => kill_cmd(args),
        "spawn" => spawn_cmd(args),
        "run"   => run_script(args),
        "yield" => { crate::scheduler::yield_now(); }
        "sleep" => sleep_cmd(args),
        "log"   => set_log_level(args),
        "mem"   => mem_cmd(),
        "getenv"=> getenv_cmd(args),
        "setenv"=> setenv_cmd(args),
        "cpuinfo" => cpuinfo_cmd(),
        "getpid" => getpid_cmd(),
        "uptime" => uptime_cmd(),
        "clear" => crate::vga::init(),
        "reboot"=> crate::power::reboot(),
        "shutdown" => crate::power::shutdown(),
        "exit"  => { crate::println!("Goodbye."); scheduler::exit_current(0); }
        other   => crate::println!("unknown command: {other}"),
    }
}

fn read_line(out: &mut String, history: &mut Vec<String>) {
    let mut history_idx = history.len();
    loop {
        crate::scheduler::yield_now();
        x86_64::instructions::hlt(); 
        if let Some(b) = crate::driver::keyboard::try_get() {
            match b {
                b'\n' | b'\r' => return,
                0x08 => { 
                    if out.pop().is_some() { 
                        crate::print!("\x08 \x08"); 
                    } 
                }
                0xE0 => { 
                    if history_idx > 0 {
                        history_idx -= 1;
                        clear_line(out);
                        out.push_str(&history[history_idx]);
                        crate::print!("{out}");
                    }
                }
                0xE1 => { 
                    if history_idx < history.len() - 1 {
                        history_idx += 1;
                        clear_line(out);
                        out.push_str(&history[history_idx]);
                        crate::print!("{out}");
                    } else if history_idx == history.len() - 1 {
                        history_idx += 1;
                        clear_line(out);
                    }
                }
                b if (0x20..0x7F).contains(&b) => {
                    out.push(b as char);
                    crate::print!("{}", b as char);
                }
                _ => {}
            }
        }
    }
}

fn clear_line(out: &mut String) {
    for _ in 0..out.len() {
        crate::print!("\x08 \x08");
    }
    out.clear();
}

fn print_help() {
    crate::println!("commands: help, ls <p>, cat <p>, write <p> <txt>, mkdir <p>, rm <p>, ps, kill <pid>, run <p>, spawn, yield, sleep <s>, log <lvl>, mem, getenv, setenv, cpuinfo, getpid, uptime, clear, reboot, shutdown");
}

fn ls_cmd(args: &[&str]) {
    let path = args.first().unwrap_or(&"/");
    let entries = crate::fs::list(path);
    if entries.is_empty() {
        crate::println!("directory empty or not found: {path}");
    } else {
        for f in entries { crate::println!(" {f}"); }
    }
}

fn cat(args: &[&str]) {
    let Some(path) = args.first() else { return crate::println!("usage: cat <path>"); };
    match crate::fs::read(path) {
        Some(data) => crate::println!("{}", core::str::from_utf8(&data).unwrap_or("<binary>")),
        None => crate::println!("no such file: {path}"),
    }
}

fn write_cmd(args: &[&str]) {
    if args.len() < 2 { return crate::println!("usage: write <path> <text>"); }
    let path = args[0];
    let body = args[1..].join(" ");
    crate::fs::write(path, body.as_bytes());
    crate::println!("wrote {} bytes to {path}", body.len());
}

fn mkdir_cmd(args: &[&str]) {
    let Some(path) = args.first() else { return crate::println!("usage: mkdir <path>"); };
    crate::fs::mkdir(path);
    crate::println!("created directory {path}");
}

fn rm(args: &[&str]) {
    let Some(path) = args.first() else { return crate::println!("usage: rm <path>"); };
    if crate::fs::delete(path) { crate::println!("deleted {path}"); }
    else { crate::println!("no such file"); }
}

fn kill_cmd(args: &[&str]) {
    let Some(pid_str) = args.first() else { return crate::println!("usage: kill <pid>"); };
    match pid_str.parse::<usize>() {
        Ok(pid) => match crate::scheduler::kill(pid) {
            Ok(_) => crate::println!("killed process {}", pid),
            Err(e) => crate::println!("failed to kill: {}", e),
        },
        Err(_) => crate::println!("invalid pid"),
    }
}

fn spawn_cmd(args: &[&str]) {
    if args.is_empty() {
        let argv = vec!["hello".into(), "world".into()];
        let pid = crate::scheduler::spawn("dummy", argv, dummy_task);
        crate::println!("Spawned dummy task with PID {}", pid);
    } else {
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let pid = crate::scheduler::spawn("custom", argv, dummy_task);
        crate::println!("Spawned custom task with PID {}", pid);
    }
}

fn dummy_task(args: Vec<String>) -> ! {
    crate::println!("[dummy] I am a spawned task!");
    crate::println!("[dummy] My arguments are: {:?}", args);
    crate::scheduler::exit_current(0);
}

fn run_script(args: &[&str]) {
    let Some(path) = args.first() else { return crate::println!("usage: run <script>"); };
    match crate::fs::read(path) {
        Some(data) => {
            let script = core::str::from_utf8(&data).unwrap_or("");
            crate::println!("Executing script: {path}");
            for line in script.lines() {
                if !line.trim().is_empty() {
                    crate::println!("{}> {}", path, line);
                    execute_command(line);
                }
            }
        }
        None => crate::println!("script not found: {path}"),
    }
}

fn sleep_cmd(args: &[&str]) {
    let secs = args.first().and_then(|s| s.parse::<u64>().ok()).unwrap_or(1);
    crate::println!("sleeping for {} seconds...", secs);
    crate::scheduler::sleep(secs);
    crate::scheduler::yield_now();
}

fn set_log_level(args: &[&str]) {
    let Some(lvl) = args.first() else { return crate::println!("usage: log <error|warn|info|debug|trace>"); };
    let level = match *lvl {
        "error" => 1,
        "warn"  => 2,
        "info"  => 3,
        "debug" => 4,
        "trace" => 5,
        _ => return crate::println!("invalid level"),
    };
    crate::logger::set_level(level);
    crate::println!("log level set to {lvl}");
}

fn mem_cmd() {
    let (total, used) = crate::allocator::stats();
    crate::println!("Heap Size: {} bytes", total);
    crate::println!("Used:      {} bytes", used);
    crate::println!("Free:      {} bytes", total - used);
}

fn getenv_cmd(args: &[&str]) {
    let Some(key) = args.first() else { return crate::println!("usage: getenv <key>"); };
    match crate::env::get(key) {
        Some(val) => crate::println!("{key}={val}"),
        None => crate::println!("variable not set"),
    }
}

fn setenv_cmd(args: &[&str]) {
    if args.len() < 2 { return crate::println!("usage: setenv <key> <value>"); }
    crate::env::set(args[0], args[1]);
    crate::println!("set {}={}", args[0], args[1]);
}

fn cpuinfo_cmd() {
    crate::println!("CPU Vendor: {}", crate::cpuid::vendor());
    crate::println!("CPU Model:  {}", crate::cpuid::brand());
}

fn getpid_cmd() {
    let pid: u64;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inlateout("rax") 12 => pid,
        );
    }
    crate::println!("Current PID: {}", pid);
}

fn uptime_cmd() {
    let ticks = crate::scheduler::get_ticks();
    let seconds = ticks / 5;
    crate::println!("Uptime: {} seconds ({} ticks)", seconds, ticks);
}
