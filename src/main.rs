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
            Ok(0) => break,
            Ok(_) => {
                buf.pop(); // 改行削除
                if buf == "exit" {
                    break;
                }
                if !buf.is_empty() {
                    invoke_cmd(&buf);
                }
            }
            Err(e) => println!("Error: {}", e),
        }
    }
}

fn invoke_cmd(buf: &str) {
    let mut inputs: Vec<&str> = buf.split('|').collect();
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
            rec_cmd(&mut inputs);
        }
        Err(_) => println!("Fork failed"),
    }
}

fn rec_cmd(inputs: &mut Vec<&str>) {
    // 残りコマンドが1つ
    if inputs.len() == 1 {
        exec_cmd(inputs[0]);
    } else {
        let (pin, pout) = pipe().unwrap();
        match fork() {
            Ok(ForkResult::Parent { child: _ }) => {
                // stdin -> pin
                close(pout).expect("Pipe Error");
                close(0).expect("Pipe Error");
                dup2(pin, 0).expect("Pipe Error");
                close(pin).expect("Pipe Error");
                // 右のコマンドをexecして死ぬ
                let input = inputs.pop().expect("Pipe Error");
                exec_cmd(input);
            }
            Ok(ForkResult::Child) => {
                // stdout -> pout
                close(pin).expect("Pipe Error");
                close(1).expect("Pipe Error");
                dup2(pout, 1).expect("Pipe Error");
                close(pout).expect("Pipe Error");
                // 左のコマンド
                inputs.pop();
                rec_cmd(inputs);
            }
            Err(e) => {
                println!("Error occurred:{}", e);
                std::process::exit(1);
            }
        }
    }
}

fn exec_cmd(input: &str) {
    let args: Vec<&str> = input.trim().split_whitespace().collect();
    let cstring_args: Vec<CString> = args
        .iter()
        .map(|s| CString::new(s.clone()).unwrap())
        .collect();
    let exec_args: Vec<&CStr> = cstring_args.iter().map(AsRef::as_ref).collect();
    execv(exec_args[0], exec_args.as_ref()).expect("Execution failed");
}
