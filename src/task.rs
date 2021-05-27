use crate::{
    gdt,
    prelude::*,
    sync::{Mutex, OnceCell},
};
use alloc::{boxed::Box, sync::Arc, vec, vec::Vec};
use core::{
    fmt, mem,
    sync::atomic::{AtomicU64, Ordering},
};
use crossbeam_queue::ArrayQueue;
use custom_debug_derive::Debug as CustomDebug;
use x86_64::registers::control::Cr3;

static TASK_QUEUE: OnceCell<ArrayQueue<Arc<Task>>> = OnceCell::uninit();
static CURRENT_TASK: OnceCell<Mutex<Arc<Task>>> = OnceCell::uninit();

pub(crate) fn init() {
    TASK_QUEUE.init_once(|| ArrayQueue::new(100));
    CURRENT_TASK.init_once(|| Mutex::new(Arc::new(Task::new(dummy_task, 0))));
}

extern "C" fn dummy_task(_task_id: TaskId, _arg: u64) {
    panic!("dummy task called;")
}

pub(crate) extern "C" fn idle_task(task_id: TaskId, arg: u64) {
    crate::println!("idle task: task_id={}, data={:x}", task_id, arg);
    crate::hlt_loop();
}

pub(crate) fn spawn(entry_point: EntryPoint, arg: u64) -> Result<()> {
    let task = Arc::new(Task::new(entry_point, arg));
    TASK_QUEUE.get().push(task).map_err(|_| ErrorKind::Full)?;
    Ok(())
}

pub(crate) fn on_interrupt() {
    let queue = TASK_QUEUE.get();
    let next_task = match queue.pop() {
        Some(task) => task,
        None => return,
    };

    unsafe {
        let (next_task, current_task) = CURRENT_TASK.get().with_lock(|current_task_slot| {
            let current_task_ptr = (&**current_task_slot) as *const Task;
            let next_task_ptr = (&*next_task) as *const Task;

            let current_task = mem::replace(current_task_slot, next_task);
            #[allow(clippy::unwrap_used)]
            queue.push(current_task).unwrap();

            #[allow(clippy::unwrap_used)]
            let next_task = next_task_ptr.as_ref().unwrap();
            #[allow(clippy::unwrap_used)]
            let current_task = current_task_ptr.as_ref().unwrap();
            (next_task, current_task)
        });

        Task::switch(&next_task, &current_task)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
#[repr(C, align(16))]
struct TaskContext {
    // offset : 0x00
    cr3: u64,
    rip: u64,
    rflags: u64,
    reserved1: u64,
    // offset : 0x20
    cs: u64,
    ss: u64,
    fs: u64,
    gs: u64,
    // offset : 0x40
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rdi: u64,
    rsi: u64,
    rsp: u64,
    rbp: u64,
    // offset : 0x80
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    // offset : 0xc0
    fxsave_area: [u8; 512],
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C, align(16))]
struct TaskStackElement {
    _dummy: [u64; 2],
}
static_assertions::const_assert_eq!(mem::size_of::<TaskStackElement>(), 16);

#[derive(CustomDebug)]
pub(crate) struct Task {
    task_id: TaskId,
    #[debug(skip)]
    ctx: Box<TaskContext>,
    #[debug(skip)]
    stack: Vec<TaskStackElement>,
}

pub(crate) type EntryPoint = extern "C" fn(arg: TaskId, arg: u64);

impl Task {
    fn new(entry_point: EntryPoint, arg: u64) -> Self {
        let task_id = TaskId::new();
        let stack_size = 1024 * 8;
        let stack_elem_size = mem::size_of::<TaskStackElement>();
        let mut task = Self {
            task_id,
            ctx: Box::new(unsafe { mem::zeroed() }),
            stack: vec![
                TaskStackElement::default();
                (stack_size + stack_elem_size - 1) / stack_elem_size
            ],
        };

        let selectors = gdt::selectors();

        task.ctx.rip = entry_point as *const u8 as u64;
        task.ctx.rdi = task_id.0;
        task.ctx.rsi = arg;

        task.ctx.cr3 = Cr3::read().0.start_address().as_u64();
        task.ctx.rflags = 0x202;
        task.ctx.cs = u64::from(selectors.kernel_code_selector.0);
        task.ctx.ss = u64::from(selectors.kernel_stack_selector.0);
        task.ctx.rsp = unsafe { (task.stack.as_ptr() as *const u8).add(stack_size - 8) as u64 };
        assert!(task.ctx.rsp & 0xf == 8);

        task.ctx.fxsave_area[24..][..4].copy_from_slice(&0x1f80u32.to_le_bytes());
        task
    }

    fn switch(next: &'static Task, current: &'static Task) {
        switch_task(&next.ctx, &current.ctx);
    }
}

#[naked]
extern "C" fn switch_task(_next: &'static TaskContext, _current: &'static TaskContext) {
    unsafe {
        asm!(
            "mov [rsi + 0x40], rax",
            "mov [rsi + 0x48], rbx",
            "mov [rsi + 0x50], rcx",
            "mov [rsi + 0x58], rdx",
            "mov [rsi + 0x60], rdi",
            "mov [rsi + 0x68], rsi",
            //
            "lea rax, [rsp + 8]",
            "mov [rsi + 0x70], rax", // RIP
            "mov [rsi + 0x78], rbp",
            //
            "mov [rsi + 0x80], r8",
            "mov [rsi + 0x88], r9",
            "mov [rsi + 0x90], r10",
            "mov [rsi + 0x98], r11",
            "mov [rsi + 0xa0], r12",
            "mov [rsi + 0xa8], r13",
            "mov [rsi + 0xb0], r14",
            "mov [rsi + 0xb8], r15",
            //
            "mov rax, cr3",
            "mov [rsi + 0x00], rax", // CR3
            "mov rax, [rsp]",
            "mov [rsi + 0x08], rax", // RIP
            "pushfq",
            "pop QWORD PTR [rsi + 0x10]", // RFLAGS
            //
            "mov ax, cs",
            "mov [rsi + 0x20], rax",
            "mov bx, ss",
            "mov [rsi + 0x28], rbx",
            "mov cx, fs",
            "mov [rsi + 0x30], rcx",
            "mov dx, gs",
            "mov [rsi + 0x38], rdx",
            //
            "fxsave [rsi + 0xc0]",
            //
            // stack frame for iret
            "push QWORD PTR [rdi + 0x28]", // SS
            "push QWORD PTR [rdi + 0x70]", // RSP
            "push QWORD PTR [rdi + 0x10]", // RFLAGS
            "push QWORD PTR [rdi + 0x20]", // CS
            "push QWORD PTR [rdi + 0x08]", // RIP
            //
            // restore context
            "fxrstor [rdi + 0xc0]",
            //
            "mov rax, [rdi + 0x00]",
            "mov cr3, rax",
            "mov rax, [rdi + 0x30]",
            "mov fs, ax",
            "mov rax, [rdi + 0x38]",
            "mov gs, ax",
            //
            "mov rax, [rdi + 0x40]",
            "mov rbx, [rdi + 0x48]",
            "mov rcx, [rdi + 0x50]",
            "mov rdx, [rdi + 0x58]",
            "mov rsi, [rdi + 0x68]",
            "mov rbp, [rdi + 0x78]",
            "mov r8,  [rdi + 0x80]",
            "mov r9,  [rdi + 0x88]",
            "mov r10, [rdi + 0x90]",
            "mov r11, [rdi + 0x98]",
            "mov r12, [rdi + 0xa0]",
            "mov r13, [rdi + 0xa8]",
            "mov r14, [rdi + 0xb0]",
            "mov r15, [rdi + 0xb8]",
            //
            "mov rdi, [rdi + 0x60]",
            //
            "iretq",
            options(noreturn)
        );
    }
}
