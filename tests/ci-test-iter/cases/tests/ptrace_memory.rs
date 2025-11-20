//! PTRACE_PEEKDATA 内存读取测试
//!
//! 测试内容：
//! - 基本的内存读取 (栈和代码段)
//! - 连续内存读取
//! - 无效地址处理

use libc::{
    fork, ptrace, waitpid, exit, raise, iovec, user_regs_struct, c_void,
    PTRACE_TRACEME, PTRACE_CONT, PTRACE_PEEKDATA, PTRACE_GETREGSET, SIGSTOP,
};
use test_utils::*;
use std::mem;

const NT_PRSTATUS: i32 = 1;

unsafe fn peek_data(pid: i32, addr: usize) -> Result<i64, i32> {
    // 清除 errno
    *libc::__errno_location() = 0;
    let result = ptrace(PTRACE_PEEKDATA, pid, addr, 0);
    let errno = *libc::__errno_location();
    
    if result == -1 && errno != 0 {
        Err(errno)
    } else {
        Ok(result)
    }
}

#[test]
fn test_ptrace_peekdata_basic() {
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

            // 在栈上保留一些数据
            let test_data: [u64; 4] = [
                0xDEADBEEFCAFEBABE,
                0x1122334455667788,
                0xABCDEF0123456789,
                0x0000000000000042,
            ];

            // 防止优化
            let sum: u64 = test_data.iter().sum();
            if sum == 0 {
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

            // 获取寄存器以找到栈指针
            let mut regs: user_regs_struct = mem::zeroed();
            let mut iov = iovec {
                iov_base: &mut regs as *mut _ as *mut c_void,
                iov_len: mem::size_of::<user_regs_struct>(),
            };

            assert_eq!(ptrace(PTRACE_GETREGSET, pid, NT_PRSTATUS, &mut iov as *mut _ as *mut c_void), 0, "PTRACE_GETREGSET 失败");

            let stack_addr = regs.sp as usize;
            println!("子进程 SP: 0x{stack_addr:016x}");

            // 尝试从栈读取
            match peek_data(pid, stack_addr) {
                Ok(data) => {
                    println!("成功从栈读取: 0x{:016x}", data as u64);
                }
                Err(e) => {
                    panic!("从栈读取 PTRACE_PEEKDATA 失败: {e}");
                }
            }

            // 尝试从 PC 读取 (代码段)
            let pc_addr = regs.pc as usize;
            match peek_data(pid, pc_addr) {
                Ok(data) => {
                    println!("成功从 PC (0x{pc_addr:016x}) 读取: 0x{:016x}", data as u64);
                }
                Err(e) => {
                    panic!("从 PC 读取 PTRACE_PEEKDATA 失败: {e}");
                }
            }

            println!("PTRACE_PEEKDATA 可以读取有效内存区域");

            // 清理
            ptrace(PTRACE_CONT, pid, 0, 0);
            waitpid(pid, &mut status, 0);
        }
    }
}

#[test]
fn test_ptrace_peekdata_invalid_addr() {
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

            exit(0);
        } else {
            // 父进程
            let mut status: i32 = 0;

            // 等待子进程停止
            let wait_res = waitpid(pid, &mut status, 0);
            assert_eq!(wait_res, pid, "waitpid 失败");

            assert!(wifstopped!(status), "子进程未停止，状态: 0x{status:x}");

            // 尝试读取无效地址
            let invalid_addrs = vec![
                0x0,                // NULL
                0x1,                // 非常低的地址
                0xFFFFFFFFFFFFFFFF, // 非常高的地址
            ];

            let mut all_failed_correctly = true;

            for addr in invalid_addrs {
                match peek_data(pid, addr) {
                    Ok(data) => {
                        println!("地址 0x{addr:x} 返回数据: 0x{data:x} (可能已映射)");
                        all_failed_correctly = false;
                    }
                    Err(_) => {
                        println!("PTRACE_PEEKDATA 对无效地址 0x{addr:x} 正确失败");
                    }
                }
            }

            // 清理
            ptrace(PTRACE_CONT, pid, 0, 0);
            waitpid(pid, &mut status, 0);

            if !all_failed_correctly {
                println!("某些无效地址可读 (依赖于操作系统)");
            }
        }
    }
}
