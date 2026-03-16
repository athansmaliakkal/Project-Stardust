/*
 * Global Descriptor Table (GDT) and Task State Segment (TSS).
 *
 * This module sets up the segmentation model for the x86_64 processor.
 * Although x86_64 is primarily a flat-memory architecture, the GDT is still
 * required for defining privilege levels (Ring 0 vs Ring 3) and for context
 * switching using the TSS.
 */

use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::instructions::segmentation::Segment; 
use x86_64::VirtAddr;
use lazy_static::lazy_static;

/// Index for the Double Fault interrupt stack in the IST.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    /// The Task State Segment (TSS) defines the stacks used during privilege
    /// transitions (e.g., when an interrupt occurs while in userspace).
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        
        // Setup the stack for Double Fault exceptions.
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            stack_start + STACK_SIZE
        };

        // Setup the Ring 0 stack for SYSCALL transitions.
        tss.privilege_stack_table[0] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            stack_start + STACK_SIZE
        };

        tss
    };
}

/// Container for the segment selectors used by the kernel.
pub struct Selectors {
    pub kernel_code: SegmentSelector,
    pub kernel_data: SegmentSelector,
    pub user_data: SegmentSelector,
    pub user_code: SegmentSelector,
    pub tss: SegmentSelector,
}

lazy_static! {
    /// The Global Descriptor Table.
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        
        // Define standard flat-model segments.
        let kernel_code = gdt.add_entry(Descriptor::kernel_code_segment());
        let kernel_data = gdt.add_entry(Descriptor::kernel_data_segment());
        let user_data = gdt.add_entry(Descriptor::user_data_segment());
        let user_code = gdt.add_entry(Descriptor::user_code_segment());
        let tss = gdt.add_entry(Descriptor::tss_segment(&TSS));
        
        (gdt, Selectors { kernel_code, kernel_data, user_data, user_code, tss })
    };
}

/// Initializes the GDT and segment registers for the bootstrap processor.
pub fn init() {
    GDT.0.load();
    unsafe {
        x86_64::instructions::segmentation::CS::set_reg(GDT.1.kernel_code);
        x86_64::instructions::segmentation::SS::set_reg(GDT.1.kernel_data);
    }
}

/// Initializes the GDT for application processors.
/// Each core must load the GDT to operate correctly in protected/long mode.
pub fn init_ap(logical_id: u32) {
    GDT.0.load();
    unsafe {
        x86_64::instructions::segmentation::CS::set_reg(GDT.1.kernel_code);
        x86_64::instructions::segmentation::SS::set_reg(GDT.1.kernel_data);
        
        // The TSS is only loaded on specific cores as part of SMP initialization.
        if logical_id == 1 {
            x86_64::instructions::tables::load_tss(GDT.1.tss);
        }
    }
}
