extern crate nix;
use nix::sys::wait::*;
use nix::unistd::*;
use std::ffi::{CStr, CString};
use std::io::{self, Write};

fn main() {
    const PROMPT: &str = "shsh $>";
    loop {
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        let mut buf = String::new();
        // is EOF?
        if io::stdin().read_line(&mut buf).expect("Input error") == 0 {
            return;
        }
        let args: Vec<&str> = buf.trim().split_whitespace().collect();
        invoke_cmd(&args);
    }
}

fn invoke_cmd(args: &Vec<&str>) {
    match fork() {
        Ok(ForkResult::Parent { child }) => match waitpid(child, None).expect("wait_pid failed") {
            WaitStatus::Exited(_, status) => {
                if status != 0 {
                    println!("Exit status: status={}", status)
                }
            }
            WaitStatus::Signaled(_, status, _) => println!("Signaled:status={}", status),
            _ => println!("Abnormal exit!"),
        },
        Ok(ForkResult::Child) => {
            if args.len() == 0 {
                return;
            }
            let cstring_args: Vec<CString> = args
                .iter()
                .map(|s| CString::new(s.clone()).unwrap())
                .collect();
            let exec_args: Vec<&CStr> = cstring_args.iter().map(AsRef::as_ref).collect();

            execv(exec_args[0], exec_args.as_ref()).expect("Execution failed");
        }
        Err(_) => println!("Fork failed"),
    }
}
