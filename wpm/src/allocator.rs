//! Аллокатор кучи с возвратом и объединением свободных блоков для WPM.
//!
//! Крупные пакеты (libcwunix, openssl, openssh) требуют десятки мегабайт при
//! распаковке. Полноценный `dealloc` с коалесценцией позволяет устанавливать
//! десятки пакетов подряд внутри 128 МиБ BSS-буфера без исчерпания памяти.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, Ordering};

const WPM_HEAP_SIZE: usize = 128 * 1024 * 1024;
const MIN_BLOCK_SIZE: usize = 32;

#[repr(C)]
struct FreeBlock {
    size: usize,
    next: *mut FreeBlock,
}

#[repr(C, align(16))]
struct AlignedHeap([u8; WPM_HEAP_SIZE]);

pub struct FreeListAllocator {
    heap: UnsafeCell<AlignedHeap>,
    head: UnsafeCell<*mut FreeBlock>,
    initialized: UnsafeCell<bool>,
    lock: AtomicBool,
}

unsafe impl Sync for FreeListAllocator {}

impl FreeListAllocator {
    #[inline]
    fn lock_heap(&self) {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
    }

    #[inline]
    fn unlock_heap(&self) {
        self.lock.store(false, Ordering::Release);
    }

    unsafe fn ensure_init(&self) {
        if !unsafe { *self.initialized.get() } {
            let base = unsafe { (*self.heap.get()).0.as_mut_ptr() } as *mut FreeBlock;
            unsafe {
                (*base).size = WPM_HEAP_SIZE & !15;
                (*base).next = null_mut();
                *self.head.get() = base;
                *self.initialized.get() = true;
            }
        }
    }
}

unsafe impl GlobalAlloc for FreeListAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() == 0 {
            return layout.align() as *mut u8;
        }

        let align = layout.align().max(16);
        let header_bytes = 2 * core::mem::size_of::<usize>();

        self.lock_heap();
        unsafe {
            self.ensure_init();

            let mut prev: *mut FreeBlock = null_mut();
            let mut curr = *self.head.get();

            while !curr.is_null() {
                let curr_addr = curr as usize;
                let curr_size = (*curr).size;
                let data_addr = (curr_addr + header_bytes + align - 1) & !(align - 1);
                let end_addr = (data_addr + layout.size() + 15) & !15;
                let needed = end_addr - curr_addr;

                if needed <= curr_size {
                    let rem = curr_size - needed;
                    let next_free = (*curr).next;
                    let used_size = if rem >= MIN_BLOCK_SIZE {
                        let split = (curr_addr + needed) as *mut FreeBlock;
                        (*split).size = rem;
                        (*split).next = next_free;
                        if prev.is_null() {
                            *self.head.get() = split;
                        } else {
                            (*prev).next = split;
                        }
                        needed
                    } else {
                        if prev.is_null() {
                            *self.head.get() = next_free;
                        } else {
                            (*prev).next = next_free;
                        }
                        curr_size
                    };

                    let meta = (data_addr as *mut usize).sub(2);
                    meta.write(curr_addr);
                    meta.add(1).write(used_size);
                    self.unlock_heap();
                    return data_addr as *mut u8;
                }

                prev = curr;
                curr = (*curr).next;
            }
        }
        self.unlock_heap();
        null_mut()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() == 0 || ptr.is_null() {
            return;
        }

        self.lock_heap();
        unsafe {
            let meta = (ptr as *const usize).sub(2);
            let block_addr = meta.read();
            let block_size = meta.add(1).read();

            let mut prev: *mut FreeBlock = null_mut();
            let mut curr = *self.head.get();
            while !curr.is_null() && (curr as usize) < block_addr {
                prev = curr;
                curr = (*curr).next;
            }

            let node = block_addr as *mut FreeBlock;
            (*node).size = block_size;
            (*node).next = curr;

            if prev.is_null() {
                *self.head.get() = node;
            } else {
                (*prev).next = node;
            }

            if !curr.is_null() && block_addr + (*node).size == curr as usize {
                (*node).size += (*curr).size;
                (*node).next = (*curr).next;
            }

            if !prev.is_null() && (prev as usize) + (*prev).size == block_addr {
                (*prev).size += (*node).size;
                (*prev).next = (*node).next;
            }
        }
        self.unlock_heap();
    }
}

#[global_allocator]
pub static ALLOCATOR: FreeListAllocator = FreeListAllocator {
    heap: UnsafeCell::new(AlignedHeap([0u8; WPM_HEAP_SIZE])),
    head: UnsafeCell::new(null_mut()),
    initialized: UnsafeCell::new(false),
    lock: AtomicBool::new(false),
};
