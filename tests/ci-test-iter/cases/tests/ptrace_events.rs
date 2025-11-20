//! PTRACE 事件追踪测试
//!
//! 测试内容：
//! - PTRACE_EVENT_FORK
//! - PTRACE_EVENT_CLONE
//! - PTRACE_EVENT_EXEC
//! - PTRACE_EVENT_EXIT
//! - PTRACE_EVENT_VFORK

use libc::{
    fork, ptrace, waitpid, exit, raise, c_void, c_char,
    PTRACE_TRACEME, PTRACE_CONT, PTRACE_SETOPTIONS, PTRACE_GETEVENTMSG,
    PTRACE_O_TRACEFORK, PTRACE_O_TRACEEXEC, PTRACE_O_TRACEEXIT,
    PTRACE_EVENT_FORK, PTRACE_EVENT_EXEC, PTRACE_EVENT_EXIT,
    SIGSTOP, SIGTRAP,
};
use test_utils::*;
use std::ffi::CString;
use std::ptr;

fn get_ptrace_event(status: i32) -> Option<i32> {
    if wifstopped!(status) && wstopsig!(status) == SIGTRAP {
        let event = (status >> 16) & 0xff;
        if event != 0 {
            Some(event)
        } else {
            None
        }
    } else {
        None
    }
}

fn is_ptrace_event(status: i32, event: i32) -> bool {
    get_ptrace_event(status) == Some(event)
}

#[test]
fn test_ptrace_event_fork_basic() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程
            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }

            if raise(SIGSTOP) == -1 {
                eprintln!("子进程: raise(SIGSTOP) 失败");
                exit(1);
            }

            // Fork - 这应该触发 PTRACE_EVENT_FORK
            let child_pid = libc::fork();
            if child_pid == 0 {
                // 孙进程 - 立即退出
                exit(0);
            }

            exit(0);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 等待子进程停止
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid (初始) 失败");

            assert!(wifstopped!(status) && wstopsig!(status) == SIGSTOP, "预期 SIGSTOP，实际状态: 0x{status:x}");

            println!("子进程停止于初始 SIGSTOP");

            // 设置 PTRACE_O_TRACEFORK 选项
            assert_eq!(ptrace(PTRACE_SETOPTIONS, pid, 0, PTRACE_O_TRACEFORK), 0, "PTRACE_SETOPTIONS 失败");

            println!("已设置 PTRACE_O_TRACEFORK 选项");

            // 继续子进程
            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT 失败");

            // 等待 PTRACE_EVENT_FORK
            let wait_res_fork = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_fork, pid, "waitpid (fork 事件) 失败");

            assert!(is_ptrace_event(status, PTRACE_EVENT_FORK), "预期 PTRACE_EVENT_FORK，实际状态: 0x{:x}, 事件: {:?}", status, get_ptrace_event(status));

            println!("收到 PTRACE_EVENT_FORK");

            // 获取新子进程 PID
            let mut msg: u64 = 0;
            assert_eq!(ptrace(PTRACE_GETEVENTMSG, pid, 0, &mut msg as *mut _ as *mut c_void), 0, "PTRACE_GETEVENTMSG 失败");
            let grandchild_pid = msg as i32;

            println!("GETEVENTMSG 返回的孙进程 PID: {grandchild_pid}");

            assert!(grandchild_pid > 0, "无效的孙进程 PID: {grandchild_pid}");

            // 继续子进程
            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT (fork 事件后) 失败");

            // 等待孙进程停止于 SIGSTOP (自动 attach)
            let wait_res_gc = waitpid(grandchild_pid, &mut status, 0);
            assert_eq!(wait_res_gc, grandchild_pid, "waitpid (孙进程) 失败");

            assert!(wifstopped!(status) && wstopsig!(status) == SIGSTOP, "预期孙进程停止于 SIGSTOP，实际状态: 0x{status:x}");

            println!("孙进程自动被追踪并停止于 SIGSTOP");

            // 继续孙进程让其退出
            assert_eq!(ptrace(PTRACE_CONT, grandchild_pid, 0, 0), 0, "PTRACE_CONT (孙进程) 失败");

            // 等待孙进程退出
            let wait_res_gc_exit = waitpid(grandchild_pid, &mut status, 0);
            assert_eq!(wait_res_gc_exit, grandchild_pid, "waitpid (孙进程退出) 失败");

            assert!(wifexited!(status), "预期孙进程退出，实际状态: 0x{status:x}");

            println!("孙进程成功退出");

            // 等待子进程退出
            loop {
                let wait_res_child = waitpid(pid, &mut status, 0);
                assert_eq!(wait_res_child, pid, "waitpid (子进程) 失败");

                if wifexited!(status) {
                    println!("子进程成功退出");
                    break;
                }

                if wifstopped!(status) {
                    // 继续 (可能是收到 SIGCHLD)
                    assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT (子进程信号) 失败");
                } else {
                    panic!("意外的子进程状态: 0x{status:x}");
                }
            }
        }
    }
}

#[test]
fn test_ptrace_event_exec() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }
            if raise(SIGSTOP) == -1 {
                eprintln!("子进程: raise(SIGSTOP) 失败");
                exit(1);
            }

            let path = CString::new("/bin/true").unwrap();
            let arg0 = CString::new("true").unwrap();
            libc::execl(path.as_ptr(), arg0.as_ptr(), ptr::null::<c_char>());
            exit(1);
        } else {
            let mut status: i32 = 0;
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid (初始) 失败");

            assert_eq!(ptrace(PTRACE_SETOPTIONS, pid, 0, PTRACE_O_TRACEEXEC), 0, "PTRACE_SETOPTIONS 失败");
            println!("已设置 PTRACE_O_TRACEEXEC 选项");

            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT 失败");

            let wait_res_exec = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_exec, pid, "waitpid (exec 事件) 失败");

            assert!(is_ptrace_event(status, PTRACE_EVENT_EXEC), "预期 PTRACE_EVENT_EXEC，实际状态: 0x{status:x}");
            println!("收到 PTRACE_EVENT_EXEC");

            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT (exec 后) 失败");

            let wait_res_exit = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_exit, pid, "waitpid (子进程退出) 失败");

            assert!(wifexited!(status), "子进程未正常退出，状态: 0x{status:x}");

            println!("exec 后子进程退出");
        }
    }
}

#[test]
fn test_ptrace_event_exit() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }
            if raise(SIGSTOP) == -1 {
                eprintln!("子进程: raise(SIGSTOP) 失败");
                exit(1);
            }
            exit(42);
        } else {
            let mut status: i32 = 0;
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid (初始) 失败");

            assert_eq!(ptrace(PTRACE_SETOPTIONS, pid, 0, PTRACE_O_TRACEEXIT), 0, "PTRACE_SETOPTIONS 失败");
            println!("已设置 PTRACE_O_TRACEEXIT 选项");

            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT 失败");

            let wait_res_exit_evt = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_exit_evt, pid, "waitpid (exit 事件) 失败");

            assert!(is_ptrace_event(status, PTRACE_EVENT_EXIT), "预期 PTRACE_EVENT_EXIT，实际状态: 0x{status:x}");
            println!("收到 PTRACE_EVENT_EXIT");

            let mut msg: u64 = 0;
            assert_eq!(ptrace(PTRACE_GETEVENTMSG, pid, 0, &mut msg as *mut _ as *mut c_void), 0, "PTRACE_GETEVENTMSG 失败");
            println!("GETEVENTMSG 返回的退出状态: {msg}");

            assert_eq!(ptrace(PTRACE_CONT, pid, 0, 0), 0, "PTRACE_CONT (exit 事件后) 失败");

            let wait_res_final = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_final, pid, "waitpid (最终) 失败");

            assert!(wifexited!(status) && wexitstatus!(status) == 42, "子进程以意外状态退出: 0x{status:x}");

            println!("子进程以预期状态 42 退出");
        }
    }
}
