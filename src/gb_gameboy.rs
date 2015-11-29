use gb_memory::*;
use gb_cpu::*;

pub struct GameBoyState {
    pub cpu: CPUState,
    pub mem: MemoryMapState
}

impl GameBoyState {
    pub fn new() -> GameBoyState {

        GameBoyState {
            cpu: CPUState::new(),
            mem: MemoryMapState::new(),

        }
    }
}
