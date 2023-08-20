/// Stores information about a running process.
pub struct Process {
    /// The physical address of the process's l4 page table.
    pub address_space: usize,
}

/// The process that's currently running.
pub static mut CURRENT_PROCESS: Process = Process { address_space: 0 };
