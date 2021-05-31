use crate::{
    co_task::{CoTask, Executor},
    gdt,
    prelude::*,
    sync::{Mutex, OnceCell},
};
use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    vec,
};
use core::{
    fmt,
    future::Future,
    mem,
    sync::atomic::{AtomicU64, AtomicUsize, Ordering},
};
use custom_debug_derive::Debug as CustomDebug;
use x86_64::{instructions::interrupts, registers::control::Cr3};

static TASK_MANAGER: OnceCell<Mutex<TaskManager>> = OnceCell::uninit();

pub(crate) fn init() {
    let main_task = Task::new_main();
    main_task.set_level(MAX_LEVEL);
    TASK_MANAGER.init_once(|| Mutex::new(TaskManager::new(main_task)));

    let idle_task = Task::new(async { crate::hlt_loop() });
    idle_task.set_level(MIN_LEVEL);
    spawn(idle_task);
}

struct EntryPointArg {
    executor: Executor,
}

extern "C" fn task_entry_point(arg: *mut EntryPointArg) {
    let EntryPointArg { mut executor } = *unsafe { Box::from_raw(arg) };
    executor.run();
}

pub(crate) fn spawn(task: Task) -> TaskId {
    assert!(!interrupts::are_enabled());

    let task = Arc::new(task);
    let task_id = task.id;

    TASK_MANAGER.get().with_lock(|task_manager| {
        task_manager.spawn(task);
        task_manager.wake(task_id);
    });

    task_id
}

pub(crate) fn wake(task_id: TaskId) {
    assert!(!interrupts::are_enabled());
    TASK_MANAGER.get().lock().wake(task_id)
}

pub(crate) fn sleep(task_id: TaskId) {
    assert!(!interrupts::are_enabled());
    if let Some(switch_task) = TASK_MANAGER.get().with_lock(|tm| tm.sleep(task_id)) {
        switch_task.switch();
    }
}

pub(crate) fn current() -> Arc<Task> {
    assert!(!interrupts::are_enabled());
    TASK_MANAGER.get().lock().current_task()
}

#[derive(Debug)]
#[must_use]
struct SwitchTask {
    next_task: Arc<Task>,
    current_task: Arc<Task>,
}

impl SwitchTask {
    fn switch(self) {
        assert!(Arc::strong_count(&self.next_task) > 1);
        assert!(Arc::strong_count(&self.current_task) > 1);
        unsafe {
            let next_task_ptr = Arc::as_ptr(&self.next_task);
            let current_task_ptr = Arc::as_ptr(&self.current_task);
            drop(self.next_task);
            drop(self.current_task);
            #[allow(clippy::unwrap_used)]
            let next_task = next_task_ptr.as_ref().unwrap();
            #[allow(clippy::unwrap_used)]
            let current_task = current_task_ptr.as_ref().unwrap();

            Task::switch(next_task, current_task)
        }
    }
}

const MAX_LEVEL: usize = 3;
const MIN_LEVEL: usize = 0;
const DEFAULT_LEVEL: usize = 1;

#[derive(Debug)]
struct TaskManager {
    tasks: BTreeMap<TaskId, Arc<Task>>,
    current_task_id: TaskId,
    wake_queue: [VecDeque<TaskId>; MAX_LEVEL + 1],
}

impl TaskManager {
    fn new(current_task: Task) -> Self {
        let current_task_id = current_task.id;
        let mut tasks = BTreeMap::new();
        tasks.insert(current_task_id, Arc::new(current_task));
        Self {
            tasks,
            current_task_id,
            wake_queue: [
                VecDeque::with_capacity(1),
                VecDeque::with_capacity(1),
                VecDeque::with_capacity(1),
                VecDeque::with_capacity(1),
            ],
        }
    }

    fn spawn(&mut self, task: Arc<Task>) {
        let task_id = task.id;
        if self.tasks.insert(task_id, task).is_some() {
            panic!("task with same ID already in tasks")
        }
        // avoid memory allocation during task switching
        for queue in self.wake_queue.iter_mut() {
            queue.reserve_exact(self.tasks.len() - queue.len());
        }
    }

    fn switch_context(&mut self, sleep_current: bool) -> Option<SwitchTask> {
        let current_task = self.current_task();
        let next_task_level = if sleep_current {
            MIN_LEVEL
        } else {
            current_task.level()
        };

        let next_task = match self.pop_next_task(next_task_level) {
            Some(next_task) if next_task.id != current_task.id => next_task,
            _ => return None, // do nothing
        };

        if !sleep_current {
            let level = current_task.level();
            self.wake_queue[level].push_back(current_task.id);
        }
        self.current_task_id = next_task.id;

        Some(SwitchTask {
            next_task,
            current_task,
        })
    }

    fn wake(&mut self, task_id: TaskId) {
        let task = match self.tasks.get(&task_id) {
            Some(task) => task,
            None => return, // finished task
        };
        let level = task.level();
        if self.current_task_id == task_id || self.wake_queue[level].contains(&task_id) {
            // already running or requested to wake
            return;
        }
        // request to wake
        self.wake_queue[level].push_back(task_id);
    }

    fn sleep(&mut self, task_id: TaskId) -> Option<SwitchTask> {
        if self.current_task_id == task_id {
            // sleep running task
            return self.switch_context(true);
        }

        let task = match self.tasks.get(&task_id) {
            Some(task) => task,
            None => return None, // finished task
        };
        let level = task.level();
        let idx = self.wake_queue[level].iter().position(|t| *t == task_id)?;
        let _ = self.wake_queue[level].remove(idx);
        None
    }

    fn current_task(&self) -> Arc<Task> {
        #[allow(clippy::unwrap_used)] // current task must be exist
        Arc::clone(self.tasks.get(&self.current_task_id).unwrap())
    }

    fn pop_next_task(&mut self, task_level: usize) -> Option<Arc<Task>> {
        for queue in self.wake_queue[task_level..].iter_mut().rev() {
            while let Some(task_id) = queue.pop_front() {
                if let Some(task) = self.tasks.get(&task_id) {
                    return Some(Arc::clone(task));
                }
            }
        }
        None
    }
}

pub(crate) fn on_interrupt() {
    if let Some(task_switch) = TASK_MANAGER.get().with_lock(|tm| tm.switch_context(false)) {
        task_switch.switch();
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

impl Default for TaskContext {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(C, align(16))]
struct TaskStackElement {
    _dummy: [u8; 16],
}
static_assertions::const_assert_eq!(mem::size_of::<TaskStackElement>(), 16);

#[derive(CustomDebug)]
pub(crate) struct Task {
    id: TaskId,
    level: AtomicUsize,
    #[debug(skip)]
    ctx: Box<TaskContext>,
    #[debug(skip)]
    _stack: Box<[TaskStackElement]>,
}

impl Task {
    fn new_main() -> Self {
        let id = TaskId::new();
        let level = AtomicUsize::new(DEFAULT_LEVEL);
        let stack = vec![].into_boxed_slice();
        let ctx = Box::new(TaskContext::default());

        Self {
            id,
            level,
            ctx,
            _stack: stack,
        }
    }

    pub(crate) fn new(future: impl Future<Output = ()> + Send + 'static) -> Self {
        let id = TaskId::new();
        let level = AtomicUsize::new(DEFAULT_LEVEL);
        let stack_size = 1024 * 8;
        let stack_elem_size = mem::size_of::<TaskStackElement>();
        let stack =
            vec![TaskStackElement::default(); (stack_size + stack_elem_size - 1) / stack_elem_size]
                .into_boxed_slice();

        let mut executor = Executor::new(id);
        executor.spawn(CoTask::new(future));
        let arg = Box::new(EntryPointArg { executor });

        let mut ctx = Box::new(TaskContext::default());

        // arguments
        ctx.rip = task_entry_point as *const u8 as u64;
        ctx.rdi = Box::into_raw(arg) as u64;

        // registers
        let selectors = gdt::selectors();
        ctx.cr3 = Cr3::read().0.start_address().as_u64();
        ctx.rflags = 0x202;
        ctx.cs = u64::from(selectors.kernel_code_selector.0);
        ctx.ss = u64::from(selectors.kernel_stack_selector.0);
        ctx.rsp = unsafe { (stack.as_ptr() as *const u8).add(stack_size - 8) as u64 };
        assert!(ctx.rsp & 0xf == 8);

        ctx.fxsave_area[24..][..4].copy_from_slice(&0x1f80u32.to_le_bytes());

        Self {
            id,
            level,
            ctx,
            _stack: stack,
        }
    }

    pub(crate) fn id(&self) -> TaskId {
        self.id
    }

    fn level(&self) -> usize {
        self.level.load(Ordering::Relaxed)
    }

    fn set_level(&self, level: usize) {
        self.level.store(level, Ordering::Relaxed);
    }

    fn switch(next: &Task, current: &Task) {
        switch_context(&next.ctx, &current.ctx);
    }
}

#[naked]
extern "C" fn switch_context(_next: &TaskContext, _current: &TaskContext) {
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
