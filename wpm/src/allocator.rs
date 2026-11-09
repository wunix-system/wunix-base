//! Аллокатор кучи для утилиты WPM.
//!
//! Обеспечивает достаточный объем памяти (32 МиБ в BSS) для безопасной
//! распаковки пакетов программ и работы с архивами данных без OOM.

const WPM_HEAP_SIZE: usize = 32 * 1024 * 1024;

pub struct SimpleBumpAllocator {
    heap: core::cell::UnsafeCell<[u8; WPM_HEAP_SIZE]>,
    offset: core::sync::atomic::AtomicUsize,
}

unsafe impl Sync for SimpleBumpAllocator {}

unsafe impl core::alloc::GlobalAlloc for SimpleBumpAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let align = layout.align();
        let size = layout.size();
        let mut cur = self.offset.load(core::sync::atomic::Ordering::Relaxed);
        loop {
            let aligned = (cur + align - 1) & !(align - 1);
            let next = aligned + size;
            if next > WPM_HEAP_SIZE {
                return core::ptr::null_mut();
            }
            match self.offset.compare_exchange_weak(
                cur,
                next,
                core::sync::atomic::Ordering::SeqCst,
                core::sync::atomic::Ordering::Relaxed,
            ) {
                Ok(_) => {
                    let ptr = self.heap.get() as *mut u8;
                    return unsafe { ptr.add(aligned) };
                }
                Err(actual) => cur = actual,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
}

#[global_allocator]
pub static ALLOCATOR: SimpleBumpAllocator = SimpleBumpAllocator {
    heap: core::cell::UnsafeCell::new([0u8; WPM_HEAP_SIZE]),
    offset: core::sync::atomic::AtomicUsize::new(0),
};
