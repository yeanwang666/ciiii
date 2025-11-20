//! PTRACE_GETREGSET 寄存器读取测试
//!
//! 测试内容：
//! - 基本的寄存器读取 (NT_PRSTATUS)
//! - 系统调用期间的寄存器读取

use libc::{
    fork, ptrace, waitpid, exit, raise, iovec, user_regs_struct, c_void,
    PTRACE_TRACEME, PTRACE_CONT, PTRACE_SYSCALL, PTRACE_GETREGSET, SIGSTOP,
};
use test_utils::*;
use std::mem;

// NT_PRSTATUS might not be exported by libc on all platforms/versions, define if missing or use libc::NT_PRSTATUS
// Usually it is 1.
const NT_PRSTATUS: i32 = 1;

#[test]
fn test_ptrace_getregs_basic() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程
            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }

            // 发送 SIGSTOP
            if raise(SIGSTOP) == -1 {
                eprintln!("子进程: raise(SIGSTOP) 失败");
                exit(1);
            }

            exit(0);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 等待子进程停止
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid 失败");

            assert!(wifstopped!(status), "子进程未停止，状态: 0x{status:x}");

            // 使用 GETREGSET 读取寄存器 (适用于 aarch64)
            let mut regs: user_regs_struct = mem::zeroed();
            let mut iov = iovec {
                iov_base: &mut regs as *mut _ as *mut c_void,
                iov_len: mem::size_of::<user_regs_struct>(),
            };

            assert_eq!(ptrace(PTRACE_GETREGSET, pid, NT_PRSTATUS, &mut iov as *mut _ as *mut c_void), 0, "PTRACE_GETREGSET 失败");

            // 基本检查
            println!("成功读取寄存器");
            println!("  PC (ELR): 0x{:016x}", regs.pc);
            println!("  SP: 0x{:016x}", regs.sp);
            // regs.regs is specific to aarch64
            #[cfg(target_arch = "aarch64")]
            println!("  X0: 0x{:016x}", regs.regs[0]);

            // PC 应该非零
            assert_ne!(regs.pc, 0, "PC 为零，可能无效");

            // SP 应该非零且对齐
            assert_ne!(regs.sp, 0, "SP 为零，可能无效");
            assert_eq!(regs.sp % 16, 0, "SP 未按 16 字节对齐: 0x{:x}", regs.sp);

            println!("寄存器值看起来合理");

            // 清理
            ptrace(PTRACE_CONT, pid, 0, 0);
            waitpid(pid, &mut status, 0);
        }
    }
}

#[test]
fn test_ptrace_getregs_during_syscall() {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork 失败");

        if pid == 0 {
            // 子进程
            if ptrace(PTRACE_TRACEME, 0, 0, 0) == -1 {
                eprintln!("子进程: PTRACE_TRACEME 失败");
                exit(1);
            }

            // 发送 SIGSTOP
            if raise(SIGSTOP) == -1 {
                eprintln!("子进程: raise(SIGSTOP) 失败");
                exit(1);
            }

            // 执行一个系统调用 (getpid)
            libc::getpid();

            exit(0);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 等待子进程停止
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid 失败");

            // 开始系统调用追踪
            assert_eq!(ptrace(PTRACE_SYSCALL, pid, 0, 0), 0, "PTRACE_SYSCALL 失败");

            // 等待系统调用入口
            let wait_res_entry = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_entry, pid, "waitpid (系统调用入口) 失败");

            assert!(wifstopped!(status), "预期在系统调用处停止，实际状态: 0x{status:x}");

            // 在系统调用入口读取寄存器
            let mut regs: user_regs_struct = mem::zeroed();
            let mut iov = iovec {
                iov_base: &mut regs as *mut _ as *mut c_void,
                iov_len: mem::size_of::<user_regs_struct>(),
            };

            assert_eq!(ptrace(PTRACE_GETREGSET, pid, NT_PRSTATUS, &mut iov as *mut _ as *mut c_void), 0, "系统调用入口处 PTRACE_GETREGSET 失败");

            #[cfg(target_arch = "aarch64")]
            {
                // On AArch64, x8 contains the syscall number
                let syscall_nr = regs.regs[8] as i64;
                println!("系统调用号 (x8): {syscall_nr}");
            }

            println!("成功读取系统调用入口处的寄存器");

            // 继续至系统调用退出
            assert_eq!(ptrace(PTRACE_SYSCALL, pid, 0, 0), 0, "PTRACE_SYSCALL (至退出) 失败");

            let wait_res_exit = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res_exit, pid, "waitpid (系统调用退出) 失败");

            // 在系统调用退出读取寄存器
            let mut regs_exit: user_regs_struct = mem::zeroed();
            let mut iov_exit = iovec {
                iov_base: &mut regs_exit as *mut _ as *mut c_void,
                iov_len: mem::size_of::<user_regs_struct>(),
            };

            assert_eq!(ptrace(PTRACE_GETREGSET, pid, NT_PRSTATUS, &mut iov_exit as *mut _ as *mut c_void), 0, "系统调用退出处 PTRACE_GETREGSET 失败");

            #[cfg(target_arch = "aarch64")]
            {
                // On AArch64, x0 contains the return value
                let retval = regs_exit.regs[0] as i64;
                println!("系统调用返回值 (x0): {retval}");
            }

            println!("成功读取系统调用入口和退出处的寄存器");

            // 清理
            ptrace(PTRACE_CONT, pid, 0, 0);
            waitpid(pid, &mut status, 0);
        }
    }
}
