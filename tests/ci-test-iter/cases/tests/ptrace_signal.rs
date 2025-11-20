//! PTRACE 信号控制测试
//!
//! 测试内容：
//! - 信号抑制 (suppression)
//! - 信号修改 (modification)

use libc::{
    fork, ptrace, waitpid, exit, raise, sigaction, sigemptyset, siginfo_t, c_void,
    PTRACE_TRACEME, PTRACE_CONT, SIGSTOP, SIGTERM, SIGUSR1, SIGUSR2, SA_SIGINFO,
};
use test_utils::*;
use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn test_ptrace_signal_suppression() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程
            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }

            // 停止以与父进程同步
            if raise(SIGSTOP) == -1 {
                eprintln!("子进程: raise(SIGSTOP) 失败");
                exit(1);
            }

            // 发送 SIGTERM - 这应该被追踪者抑制
            if raise(SIGTERM) == -1 {
                eprintln!("子进程: raise(SIGTERM) 失败");
                exit(1);
            }

            // 如果到达这里，信号被成功抑制
            exit(42);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 等待初始 SIGSTOP
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid (SIGSTOP) 失败");

            assert!(wifstopped!(status) && wstopsig!(status) == SIGSTOP, "预期 SIGSTOP，实际状态: 0x{status:x}");

            println!("子进程停止于 SIGSTOP");

            // 继续子进程，使其发送 SIGTERM
            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT (1) 失败");

            // 等待子进程在 SIGTERM 信号投递停止处停止
            let wait_res_term = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_term, pid, "waitpid (SIGTERM) 失败");

            assert!(wifstopped!(status) && wstopsig!(status) == SIGTERM, "预期 SIGTERM 停止，实际状态: 0x{status:x}");

            println!("子进程停止于 SIGTERM 信号投递停止");

            // 关键：传递 data=0 以抑制信号
            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT (抑制) 失败");

            println!("PTRACE_CONT 使用 data=0 抑制信号");

            // 等待子进程退出
            let wait_res_exit = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_exit, pid, "waitpid (退出) 失败");

            // 子进程应该正常退出 (未被 SIGTERM 杀死)
            assert!(wifexited!(status), "预期子进程正常退出，实际状态: 0x{status:x}");

            let exit_code = wexitstatus!(status);
            assert_eq!(exit_code, 42, "预期退出码 42 (信号已抑制)，实际: {exit_code}");

            println!("信号成功抑制 - 子进程正常退出");
        }
    }
}

// 信号修改测试的信号处理函数和标志
static SIGUSR1_RECEIVED: AtomicBool = AtomicBool::new(false);
static SIGUSR2_RECEIVED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigusr1_modify(
    _sig: i32,
    _info: *mut siginfo_t,
    _ctx: *mut c_void,
) {
    SIGUSR1_RECEIVED.store(true, Ordering::SeqCst);
}

extern "C" fn handle_sigusr2_modify(
    _sig: i32,
    _info: *mut siginfo_t,
    _ctx: *mut c_void,
) {
    SIGUSR2_RECEIVED.store(true, Ordering::SeqCst);
}

#[test]
fn test_ptrace_signal_modification() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程 - 设置信号处理函数
            let mut sa1: sigaction = mem::zeroed();
            sa1.sa_sigaction = handle_sigusr1_modify as usize;
            sa1.sa_flags = SA_SIGINFO;
            sigemptyset(&mut sa1.sa_mask);
            if sigaction(SIGUSR1, &sa1, ptr::null_mut()) == -1 {
                eprintln!("子进程: sigaction(SIGUSR1) 失败");
                exit(1);
            }

            let mut sa2: sigaction = mem::zeroed();
            sa2.sa_sigaction = handle_sigusr2_modify as usize;
            sa2.sa_flags = SA_SIGINFO;
            sigemptyset(&mut sa2.sa_mask);
            if sigaction(SIGUSR2, &sa2, ptr::null_mut()) == -1 {
                eprintln!("子进程: sigaction(SIGUSR2) 失败");
                exit(1);
            }

            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }

            if raise(SIGSTOP) == -1 {
                eprintln!("子进程: raise(SIGSTOP) 失败");
                exit(1);
            }

            // 发送 SIGUSR1 - 追踪者将其修改为 SIGUSR2
            if raise(SIGUSR1) == -1 {
                eprintln!("子进程: raise(SIGUSR1) 失败");
                exit(1);
            }

            // 检查收到了哪个信号
            let usr1 = SIGUSR1_RECEIVED.load(Ordering::SeqCst);
            let usr2 = SIGUSR2_RECEIVED.load(Ordering::SeqCst);

            if usr2 && !usr1 {
                exit(88); // 成功: 收到 SIGUSR2
            } else if usr1 {
                exit(11); // 错误: 收到 SIGUSR1 (未修改)
            } else {
                exit(22); // 错误: 未收到信号
            }
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 等待初始 SIGSTOP
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid (SIGSTOP) 失败");

            println!("子进程停止于 SIGSTOP");

            // 继续子进程，使其发送 SIGUSR1
            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT (1) 失败");

            // 等待子进程在 SIGUSR1 处停止
            let wait_res_usr1 = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_usr1, pid, "waitpid (SIGUSR1) 失败");

            assert!(wifstopped!(status) && wstopsig!(status) == SIGUSR1, "预期 SIGUSR1 停止，实际状态: 0x{status:x}");

            println!("子进程停止于 SIGUSR1 信号投递停止");

            // 关键：传递 SIGUSR2 以修改信号
            assert_eq!(ptrace(PTRACE_CONT, pid, 0, SIGUSR2), 0, "PTRACE_CONT (修改) 失败");

            println!("PTRACE_CONT 使用 SIGUSR2 修改信号");

            // 子进程应该再次停止于修改后的信号 (SIGUSR2)
            // 注意：这取决于内核行为，有些内核可能直接投递，有些会再次停止
            // 在 Linux 上，如果修改了信号，通常会再次触发信号停止，或者直接投递
            // 如果再次停止：
            let wait_res_usr2 = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_usr2, pid, "waitpid (SIGUSR2) 失败");

            if wifstopped!(status) && wstopsig!(status) == SIGUSR2 {
                println!("子进程停止于 SIGUSR2 (修改后的信号)");
                // 继续以投递 SIGUSR2 到处理函数
                assert_eq!(ptrace(PTRACE_CONT, pid, 0, SIGUSR2), 0, "PTRACE_CONT (投递) 失败");
                
                // 等待退出
                let wait_res_exit = waitpid(pid, &mut status, 0);
                assert_eq!(wait_res_exit, pid, "waitpid (退出) 失败");
            } else if wifexited!(status) {
                // 如果直接退出了，说明信号已经投递并处理完了
                // 这种情况也可能发生，取决于 ptrace 实现细节
            } else {
                 panic!("预期 SIGUSR2 停止或退出，实际状态: 0x{status:x}");
            }

            assert!(wifexited!(status), "预期子进程退出，实际状态: 0x{status:x}");

            let exit_code = wexitstatus!(status);
            assert_eq!(exit_code, 88, "预期退出码 88 (收到 SIGUSR2)，实际: {exit_code}");

            println!("信号成功从 SIGUSR1 修改为 SIGUSR2");
        }
    }
}
