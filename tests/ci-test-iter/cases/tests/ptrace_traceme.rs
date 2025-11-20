//! PTRACE_TRACEME 功能测试
//!
//! 测试内容：
//! - 基本的 PTRACE_TRACEME 功能
//! - 验证 PTRACE_TRACEME 会将进程标记为被追踪状态

use libc::{
    fork, ptrace, waitpid, exit, raise,
    PTRACE_TRACEME, PTRACE_CONT, SIGSTOP,
};
use test_utils::*;

#[test]
fn test_ptrace_traceme_basic() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程
            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }

            // 发送 SIGSTOP 通知父进程我们已准备好
            if raise(SIGSTOP) == -1 {
                eprintln!("子进程: raise(SIGSTOP) 失败");
                exit(1);
            }

            // 成功退出
            exit(42);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 等待子进程停止
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid 失败");

            assert!(wifstopped!(status), "预期子进程停止，实际状态: 0x{status:x}");

            let sig = wstopsig!(status);
            assert_eq!(sig, SIGSTOP, "预期 SIGSTOP，实际信号: {sig}");

            // 继续子进程
            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT 失败");

            // 等待子进程退出
            let wait_res_exit = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_exit, pid, "waitpid (退出) 失败");

            assert!(wifexited!(status), "预期子进程退出，实际状态: 0x{status:x}");

            let exit_code = wexitstatus!(status);
            assert_eq!(exit_code, 42, "预期退出码 42，实际: {exit_code}");
        }
    }
}

#[test]
fn test_ptrace_traceme_marks_traced() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程
            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }

            // 尝试再次调用 PTRACE_TRACEME - 应该失败
            if ptrace(PTRACE_TRACEME, 0, 0, 0) != -1 {
                eprintln!("子进程: 第二次 PTRACE_TRACEME 应该失败");
                exit(1);
            }
            
            // 预期 - 进程已被追踪
            exit(0);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 等待子进程退出
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid 失败");

            assert!(wifexited!(status), "预期子进程退出，实际状态: 0x{status:x}");

            let exit_code = wexitstatus!(status);
            assert_eq!(exit_code, 0, "预期退出码 0 (重复 traceme 应失败)，实际: {exit_code}");
        }
    }
}
