#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
#[cfg(target_os = "none")]
use core::alloc::{GlobalAlloc, Layout};
#[cfg(target_os = "none")]
use core::panic::PanicInfo;

use kittens_code_core::compact::CompactionState;
use kittens_code_core::record::{
    DecodeOutcome, LogHeader, Record, RecordKind, RecordPayload, scan_records,
};
use kittens_code_core::rlm::exec::Executor;
use kittens_code_core::rlm::lower::lower_script;
use kittens_code_core::tokens::TokenAccounting;
use kittens_code_core::window::WindowLayout;
use kittens_code_protocol::budgets::Budgets;
use kittens_code_protocol::config::{SessionConfig, SessionConfigPatch};
use kittens_code_protocol::event::Event;
use kittens_code_protocol::ids::{EffectId, SessionId, TurnEpoch};
use kittens_code_protocol::op::Op;

#[cfg(target_os = "none")]
struct NullAllocator;

#[cfg(target_os = "none")]
unsafe impl GlobalAlloc for NullAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[cfg(target_os = "none")]
#[global_allocator]
static ALLOCATOR: NullAllocator = NullAllocator;

fn linked_code_path() {
    let session_id = SessionId([0x4b; 16]);
    let effect_id = EffectId(7);
    let op = Op::Interrupt;
    let event = Event::ToolStarted { call: effect_id };

    let budgets = Budgets::default();
    let mut config = SessionConfig::default();
    let mut patch = SessionConfigPatch::default();
    patch.budgets = Some(budgets);
    config.apply(patch);

    let layout = WindowLayout::new(
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        Vec::new(),
        String::new(),
        Vec::new(),
    )
    .expect("the fixture tail is empty and therefore atomic");
    let mut compaction = CompactionState::default();
    let decision = compaction.decide(76, &config.compaction);
    let estimate = TokenAccounting::default().estimate_tail(4_096);

    let header = Record::new(
        0,
        RecordKind::Header,
        None,
        TurnEpoch(0),
        RecordPayload::Header(LogHeader {
            session_id,
            parent: None,
            schema_epoch: 1,
            prompt_pack_version: [0, 8, 0],
            verb_grammar_version: [0, 8, 0],
            l3_dialect_version: [1, 0, 0],
            codec: String::new(),
            created_at: None,
        }),
    )
    .expect("the fixture header kind matches its payload");
    let scanned = scan_records([DecodeOutcome::Good(header)], 1)
        .expect("the fixture header is valid for schema epoch one");

    let query = lower_script("final \"linked\"");
    let mut executor = Executor::new(query, budgets);
    let step = executor.step();

    core::hint::black_box((op, event, config, layout, decision, estimate, scanned, step));
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    if core::hint::black_box(false) {
        linked_code_path();
    }

    loop {
        core::hint::spin_loop();
    }
}

#[cfg(not(target_os = "none"))]
fn main() {
    linked_code_path();
}
