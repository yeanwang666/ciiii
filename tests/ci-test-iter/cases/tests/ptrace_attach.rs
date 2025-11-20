//! PTRACE_ATTACH 测试
//!
//! 测试内容：
//! - 基本的 attach 和 detach
//! - attach 后 continue
//! - attach 后读取寄存器并 detach

use libc::{
    fork, kill, ptrace, waitpid, exit, iovec, user_regs_struct, c_void,
    PTRACE_ATTACH, PTRACE_DETACH, PTRACE_CONT, PTRACE_GETREGSET, SIGSTOP, SIGKILL,
};
use test_utils::*;
use std::mem;
use std::thread;
use std::time::Duration;

const NT_PRSTATUS: i32 = 1;

#[test]
fn test_ptrace_attach_basic() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程 - 循环直到被杀死
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 给子进程一点时间启动
            thread::sleep(Duration::from_millis(10));

            // Attach 到子进程
            assert_eq!(ptrace(PTRACE_ATTACH, pid, 0, 0), 0, "PTRACE_ATTACH 失败");

            println!("已发出 PTRACE_ATTACH");

            // 等待子进程停止 (它会被发送 SIGSTOP)
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid 失败");

            assert!(wifstopped!(status) && wstopsig!(status) == SIGSTOP, "预期 SIGSTOP，实际状态: 0x{status:x}");

            println!("attach 后子进程停止于 SIGSTOP");

            // Detach 子进程
            assert_eq!(ptrace(PTRACE_DETACH, pid, 0, 0), 0, "PTRACE_DETACH 失败");

            println!("PTRACE_DETACH 成功");

            // 清理子进程
            assert_eq!(kill(pid, SIGKILL), 0, "kill 失败");

            println!("子进程已清理");
        }
    }
}

#[test]
fn test_ptrace_attach_and_cont() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程
            exit(55);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // Attach 到子进程
            assert_eq!(ptrace(PTRACE_ATTACH, pid, 0, 0), 0, "PTRACE_ATTACH 失败");

            println!("已发出 PTRACE_ATTACH");

            // 等待子进程停止
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid 等待 attach 停止失败");

            assert!(wifstopped!(status) && wstopsig!(status) == SIGSTOP, "预期 SIGSTOP，实际状态: 0x{status:x}");

            println!("子进程停止于 SIGSTOP");

            // 继续子进程
            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT 失败");

            println!("已发出 PTRACE_CONT");

            // 等待子进程退出
            let wait_res_exit = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_exit, pid, "waitpid 等待退出失败");

            assert!(wifexited!(status), "预期子进程退出，实际状态: 0x{status:x}");

            let exit_code = wexitstatus!(status);
            assert_eq!(exit_code, 55, "预期退出码 55，实际: {exit_code}");

            println!("子进程以预期代码 55 退出");
        }
    }
}

#[test]
fn test_ptrace_attach_getregs_detach() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程
            exit(66);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // Attach 到子进程
            assert_eq!(ptrace(PTRACE_ATTACH, pid, 0, 0), 0, "PTRACE_ATTACH 失败");

            println!("已发出 PTRACE_ATTACH");

            // 等待子进程停止
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid 等待 attach 停止失败");

            assert!(wifstopped!(status) && wstopsig!(status) == SIGSTOP, "预期 SIGSTOP，实际状态: 0x{status:x}");

            println!("子进程停止于 SIGSTOP");

            // 获取寄存器
            let mut regs: user_regs_struct = mem::zeroed();
            let mut iov = iovec {
                iov_base: &mut regs as *mut _ as *mut c_void,
                iov_len: mem::size_of::<user_regs_struct>(),
            };

            assert_eq!(ptrace(PTRACE_GETREGSET, pid, NT_PRSTATUS, &mut iov as *mut _ as *mut c_void), 0, "PTRACE_GETREGSET 失败");

            println!("PTRACE_GETREGSET 成功");
            assert_eq!(iov.iov_len, mem::size_of::<user_regs_struct>(), "PTRACE_GETREGSET 返回意外大小: {}", iov.iov_len);

            // Detach 子进程，这应该会让它恢复运行
            assert_eq!(ptrace(PTRACE_DETACH, pid, 0, 0), 0, "PTRACE_DETACH 失败");

            println!("已发出 PTRACE_DETACH");

            // 等待子进程退出
            let wait_res_exit = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_exit, pid, "waitpid 等待退出失败");

            assert!(wifexited!(status), "预期子进程退出，实际状态: 0x{status:x}");

            let exit_code = wexitstatus!(status);
            assert_eq!(exit_code, 66, "预期退出码 66，实际: {exit_code}");

            println!("detach 后子进程以预期代码 66 退出");
        }
    }
}
