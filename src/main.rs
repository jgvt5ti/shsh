extern crate nix;

use nix::sys::wait::*;
use nix::unistd::*;
use std::env;
use std::ffi::{CStr, CString};

fn main() {
    match fork() {
        Ok(ForkResult::Parent { child }) => match waitpid(child, None).expect("wait_pid failed") {
            WaitStatus::Exited(pid, status) => {
                println!("exit!: pid={:?}, status={:?}", pid, status)
            }
            WaitStatus::Signaled(pid, status, _) => {
                println!("signal!: pid={:?}, status={:?}", pid, status)
            }
            _ => println!("abnormal exit!"),
        },
        Ok(ForkResult::Child) => {
            let args: Vec<String> = env::args().collect();
            if args.len() < 2 {
                println!("Invalid argument");
                return;
            }
            let cstring_args: Vec<CString> = args
                .iter()
                .map(|s| CString::new(s.clone()).unwrap())
                .collect();
            let mut exec_args: Vec<&CStr> = cstring_args.iter().map(AsRef::as_ref).collect();
            exec_args.remove(0);

            execv(exec_args[0], exec_args.as_ref()).expect("Execution failed");
        }
        Err(_) => println!("Fork failed"),
    }
}
