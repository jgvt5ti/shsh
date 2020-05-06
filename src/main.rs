extern crate libc;
extern crate nix;
use libc as lc;
use nix::sys::wait::*;
use nix::unistd::*;
use std::env;
use std::ffi::{CStr, CString};
use std::io::{self, BufRead, Write};
use std::path::Path;

fn main() {
    loop {
        print_prompt();
        let mut buf = String::new();
        let stdin = io::stdin();
        let mut handle = stdin.lock();
        match handle.read_line(&mut buf) {
            Ok(0) => break,
            Ok(_) => {
                buf.pop(); // 改行削除
                if buf == "exit" {
                    std::process::exit(0);
                } else if buf == "help" {
                    print_help();
                } else if !buf.is_empty() {
                    let bk = backup();
                    invoke_cmd(check_redirect(&buf));
                    dup2(bk.0, 0).expect("dup2 failed");
                    dup2(bk.1, 1).expect("dup2 failed");
                    close(bk.0).expect("close failed");
                    close(bk.1).expect("close failed");
                }
            }
            Err(e) => println!("Error: {}", e),
        }
        io::stdout().flush().unwrap();
    }
}

fn print_help() {
    println!("シェルシェル");
    println!("shsh 1.0");
}

fn print_prompt() {
    print!(
        "\x1b[00;33m{}\x1b[m\x1b[00;31m@\x1b[00m{} > ",
        env::var("USER").unwrap(),
        env::current_dir().unwrap().display()
    );
    io::stdout().flush().unwrap();
}

fn backup() -> (i32, i32) {
    (dup(0).unwrap(), dup(1).unwrap())
}

enum FE<'a> {
    NoFile,
    FilePath(&'a str),
}

fn check_redirect(buf: &str) -> String {
    let mut inputs: Vec<&str> = buf.trim().split_whitespace().collect();
    let mut red_in = FE::NoFile;
    let mut red_out = FE::NoFile;
    let mut rm_list: Vec<usize> = Vec::new();
    for i in 0..inputs.len() {
        let token = inputs[i];
        if token == "<" && inputs.get(i + 1) != None {
            red_in = FE::FilePath(inputs[i + 1]);
            rm_list.push(i);
            rm_list.push(i + 1);
        } else if token == ">" && inputs.get(i + 1) != None {
            red_out = FE::FilePath(inputs[i + 1]);
            rm_list.push(i);
            rm_list.push(i + 1);
        }
    }
    let mut count: usize = 0;
    for i in rm_list {
        inputs.remove(i - count);
        count += 1;
    }
    if let FE::FilePath(path) = red_in {
        if Path::is_file(Path::new(path)) {
            redirect_in(path);
        }
    }
    if let FE::FilePath(path) = red_out {
        let p = Path::new(path);
        if !p.exists() {
            std::fs::File::create(path).expect("Can't create a file");
        }
        redirect_out(path);
    }
    let mut s = String::new();
    for (i, token) in inputs.iter().enumerate() {
        if i != 0 {
            s.push(' ');
        }
        s.push_str(token);
    }
    s
}

fn redirect_in(path: &str) {
    let path = CString::new(path).expect("CString::new failed");
    unsafe {
        let fd = lc::open(path.as_ptr(), lc::O_RDONLY);
        close(0).unwrap();
        dup2(fd, 0).expect("msg");
        close(fd).unwrap();
    }
}

fn redirect_out(path: &str) {
    let path = CString::new(path).expect("CString::new failed");
    unsafe {
        let fd = lc::open(path.as_ptr(), lc::O_WRONLY);
        close(1).unwrap();
        dup2(fd, 1).expect("msg");
        close(fd).unwrap();
    }
}

fn invoke_cmd(buf: String) {
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
                close(pout).unwrap();
                close(0).unwrap();
                dup2(pin, 0).unwrap();
                close(pin).unwrap();
                // 右のコマンドをexecして死ぬ
                let input = inputs.pop().unwrap();
                exec_cmd(input);
            }
            Ok(ForkResult::Child) => {
                // stdout -> pout
                close(pin).unwrap();
                close(1).unwrap();
                dup2(pout, 1).unwrap();
                close(pout).unwrap();
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
    if args[0] == "cd" {
        if args.len() == 1 {
            env::set_current_dir(env::var("HOME").unwrap()).unwrap();
        } else {
            env::set_current_dir(args[1]).unwrap();
        }
    } else {
        path_exec(args[0], &exec_args);
        execv(exec_args[0], exec_args.as_ref()).expect("Execution failed");
    }
}

// $PATHからnameを実行
fn path_exec(name: &str, args: &Vec<&CStr>) {
    let path_str = env::var("PATH").unwrap();
    let paths: Vec<&str> = path_str.split(':').collect();
    for path in paths {
        let full_path = format!("{}/{}", path, name);
        let p = Path::new(&full_path);
        if Path::is_file(p) {
            let full_path_c = CString::new(full_path).unwrap();
            execv(&full_path_c, args.as_ref()).expect("Execution failed");
        }
    }
}
