use gdbstub::common::Tid;
use gdbstub::target;
use gdbstub::target::ext::base::multithread::MultiThreadBase;
use gdbstub::target::ext::base::single_register_access::SingleRegisterAccessOps;
use gdbstub::target::TargetResult;

use super::XousTarget;

impl MultiThreadBase for XousTarget {
    fn read_registers(
        &mut self,
        regs: &mut gdbstub_arch::riscv::reg::RiscvCoreRegs<u32>,
        tid: Tid,
    ) -> TargetResult<(), Self> {
        let Some(pid) = self.pid else {
            for entry in regs.x.iter_mut() {
                *entry = 0;
            }
            return Ok(());
        };

        crate::services::SystemServices::with(|system_services| {
            let current_pid = system_services.current_pid();
            // Actiavte the debugging process and iterate through it,
            // noting down each active thread.
            let debugging_pid = pid;
            system_services
                .get_process(debugging_pid)
                .unwrap()
                .activate()
                .unwrap();
            let process = crate::arch::process::Process::current();
            let thread = process.thread(tid.get());
            regs.x[0] = 0;
            for (dbg_reg, thr_reg) in regs.x[1..].iter_mut().zip(thread.registers.iter()) {
                *dbg_reg = (*thr_reg) as u32;
            }
            regs.pc = (thread.sepc) as u32;

            // Restore the previous PID
            system_services
                .get_process(current_pid)
                .unwrap()
                .activate()
                .unwrap();
        });
        Ok(())
    }

    fn write_registers(
        &mut self,
        regs: &gdbstub_arch::riscv::reg::RiscvCoreRegs<u32>,
        tid: Tid,
    ) -> TargetResult<(), Self> {
        let Some(pid) = self.pid else {
            return Ok(())
        };

        crate::services::SystemServices::with(|system_services| {
            let current_pid = system_services.current_pid();

            // Actiavte the debugging process and iterate through it,
            // noting down each active thread.
            let debugging_pid = pid;
            system_services
                .get_process(debugging_pid)
                .unwrap()
                .activate()
                .unwrap();
            let mut process = crate::arch::process::Process::current();
            let thread = process.thread_mut(tid.get());
            for (thr_reg, dbg_reg) in thread.registers.iter_mut().zip(regs.x[1..].iter()) {
                *thr_reg = (*dbg_reg) as usize;
            }
            thread.sepc = (regs.pc) as usize;

            // Restore the previous PID
            system_services
                .get_process(current_pid)
                .unwrap()
                .activate()
                .unwrap();
        });
        Ok(())
    }

    fn read_addrs(
        &mut self,
        start_addr: u32,
        data: &mut [u8],
        _tid: Tid, // same address space for each core
    ) -> TargetResult<(), Self> {
        let current_addr = start_addr as usize;
        let Some(pid) = self.pid else {
            for entry in data.iter_mut() { *entry = 0 };
            return Ok(());
        };

        crate::services::SystemServices::with(|system_services| {
            let current_pid = system_services.current_pid();

            // Actiavte the debugging process and iterate through it,
            // noting down each active thread.
            let debugging_pid = pid;
            system_services
                .get_process(debugging_pid)
                .unwrap()
                .activate()
                .unwrap();
            for (offset, b) in data.iter_mut().enumerate() {
                *b = crate::arch::mem::peek_memory((current_addr + offset) as *mut u8)
                    .unwrap_or(0xff);
                // println!("<< Peek {:02x} @ {:08x}", *b, current_addr);
            }

            // Restore the previous PID
            system_services
                .get_process(current_pid)
                .unwrap()
                .activate()
                .unwrap();
        });
        Ok(())
    }

    fn write_addrs(
        &mut self,
        start_addr: u32,
        data: &[u8],
        _tid: Tid, // all threads share the same process memory space
    ) -> TargetResult<(), Self> {
        let mut current_addr = start_addr;
        let Some(pid) = self.pid else {
            println!("Couldn't poke memory: no current process!");
            return Ok(());
        };
        crate::services::SystemServices::with(|system_services| {
            let current_pid = system_services.current_pid();

            // Actiavte the debugging process and iterate through it,
            // noting down each active thread.
            let debugging_pid = pid;
            system_services
                .get_process(debugging_pid)
                .unwrap()
                .activate()
                .unwrap();
            data.iter().for_each(|b| {
                if let Err(_e) = crate::arch::mem::poke_memory(current_addr as *mut u8, *b) {
                    // panic!("couldn't poke memory: {:?}", _e);
                }
                // println!("Poked {:02x} @ {:08x}", *b, current_addr);
                current_addr += 1;
            });

            // Restore the previous PID
            system_services
                .get_process(current_pid)
                .unwrap()
                .activate()
                .unwrap();
        });
        Ok(())
    }

    #[inline(always)]
    fn list_active_threads(
        &mut self,
        register_thread: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        let Some(pid) = self.pid else {
            return Ok(());
        };
        crate::services::SystemServices::with(|system_services| {
            let current_pid = system_services.current_pid();

            let debugging_pid = pid;

            // Actiavte the debugging process and iterate through it,
            // noting down each active thread.
            system_services
                .get_process(debugging_pid)
                .unwrap()
                .activate()
                .unwrap();
            crate::arch::process::Process::current().for_each_thread_mut(|tid, _thr| {
                register_thread(Tid::new(tid).unwrap());
            });

            // Restore the previous PID
            system_services
                .get_process(current_pid)
                .unwrap()
                .activate()
                .unwrap();
        });
        Ok(())
    }

    fn support_single_register_access(&mut self) -> Option<SingleRegisterAccessOps<'_, Tid, Self>> {
        Some(self)
    }

    fn support_resume(
        &mut self,
    ) -> Option<target::ext::base::multithread::MultiThreadResumeOps<'_, Self>> {
        Some(self)
    }
}
