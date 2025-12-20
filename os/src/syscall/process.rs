//! Process management syscalls
use core::mem::size_of;

use crate::{
    mm::{MapPermission, VirtAddr},
    syscall::syscall_id_map_idx,
    task::{
        change_program_brk, current_tcb, current_tcb_mut, exit_current_and_run_next, get_count,
        suspend_current_and_run_next,
    },
    timer::get_time_us, // timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        let sec_base_addr = &mut (*ts).sec as *mut usize as *mut u8;
        let usec_base_addr = &mut (*ts).usec as *mut usize as *mut u8;
        let sec = us / 1000000;
        let usec = us % 1000000;
        let sec_bytes = sec.to_ne_bytes();
        let usec_bytes = usec.to_ne_bytes();

        for i in 0..size_of::<usize>() {
            sys_trace(1, sec_base_addr.add(i) as usize, sec_bytes[i] as usize);
            sys_trace(1, usec_base_addr.add(i) as usize, usec_bytes[i] as usize);
        }
    }
    0
}

pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    // return -1;
    match trace_request {
        0 => {
            let va = VirtAddr(id);
            let r = current_tcb(|tcb| {
                if let Some(pte) = tcb.memory_set.translate(va.floor()) {
                    if pte.is_valid() && pte.user_accessible() && pte.readable() {
                        return pte.ppn().get_bytes_array()[va.page_offset()] as isize;
                    }
                }
                -1
            });
            r
        }
        1 => {
            let va = VirtAddr(id);
            let r = current_tcb_mut(|tcb| {
                if let Some(pte) = tcb.memory_set.translate(va.floor()) {
                    if pte.is_valid() && pte.user_accessible() && pte.writable() {
                        pte.ppn().get_bytes_array()[va.page_offset()] = data as u8;
                        return 0;
                    }
                }
                -1
            });
            r
        }
        2 => {
            let t = get_count(syscall_id_map_idx(id)) as isize;
            t
        }
        _ => -1,
    }
    // -1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }

    if (prot & !0x7) != 0 || prot & 0x7 == 0 {
        return -1;
    }

    let end_va = VirtAddr(start + len);
    let has_confilict = current_tcb(|tcb| {
        for vpn in start_va.floor().0..end_va.ceil().0 {
            if let Some(pte) = tcb.memory_set.translate(vpn.into()) {
                if pte.is_valid() {
                    return true;
                }
            }
        }
        false
    });
    if has_confilict {
        return -1;
    }

    let mut permission = MapPermission::U;
    if (prot & 0x1) != 0 {
        permission |= MapPermission::R;
    }
    if (prot & 0x2) != 0 {
        permission |= MapPermission::W;
    }
    if (prot & 0x4) != 0 {
        permission |= MapPermission::X;
    }
    current_tcb_mut(|tcb| {
        tcb.memory_set
            .insert_framed_area(start_va, end_va, permission);
    });

    0
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr(start + len);

    let unmapped = current_tcb(|tcb| {
        for vpn in start_va.floor().0..end_va.ceil().0 {
            if let Some(pte) = tcb.memory_set.translate(vpn.into()) {
                if !pte.is_valid() {
                    return true;
                }
            } else {
                return true;
            }
        }
        false
    });
    if unmapped {
        return -1;
    }

    let succ = current_tcb_mut(|tcb| tcb.memory_set.unmap_area(start_va));
    if succ {
        return 0;
    }

    -1
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
