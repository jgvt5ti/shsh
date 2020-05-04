extern crate nix;
use nix::sys::wait::*;
use nix::unistd::*;
use std::ffi::{CStr, CString};
use std::io::{self, BufRead, Write};

fn main() {
    const PROMPT: &str = "ｼｪﾙｼｪﾙ $>";
    loop {
        print!("{}", PROMPT);
        io::stdout().flush().unwrap();
        let mut buf = String::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        match handle.read_line(&mut buf) {
            Ok(0) => break, // is EOF?
            Ok(_) => {
                let args: Vec<&str> = buf.trim().split_whitespace().collect();
                invoke_cmd(&args);
            }
            Err(e) => println!("Error: {}", e),
        }
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
            if args.len() != 0 {
                let cstring_args: Vec<CString> = args
                    .iter()
                    .map(|s| CString::new(s.clone()).unwrap())
                    .collect();
                let exec_args: Vec<&CStr> = cstring_args.iter().map(AsRef::as_ref).collect();

                execv(exec_args[0], exec_args.as_ref()).expect("Execution failed");
            }
            std::process::exit(0);
        }
        Err(_) => println!("Fork failed"),
    }
}
