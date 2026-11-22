use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct IntMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct IntMutexGuard<'a, T> {
    mutex: &'a IntMutex<T>,
    interrupts_enabled: bool,
}

unsafe impl<T: Send> Send for IntMutex<T> {}
unsafe impl<T: Send> Sync for IntMutex<T> {}

impl<T> IntMutex<T> {
    pub const fn new(val: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(val),
        }
    }

    pub fn lock(&self) -> IntMutexGuard<'_, T> {
        let interrupts_enabled = x86_64::instructions::interrupts::are_enabled();
        
        if interrupts_enabled {
            x86_64::instructions::interrupts::disable();
        }

        while self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            while self.locked.load(Ordering::Relaxed) {
                x86_64::instructions::halt();
            }
        }

        IntMutexGuard {
            mutex: self,
            interrupts_enabled,
        }
    }
}

impl<'a, T> Drop for IntMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.locked.store(false, Ordering::Release);
        if self.interrupts_enabled {
            x86_64::instructions::interrupts::enable();
        }
    }
}

impl<T> core::ops::Deref for IntMutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> core::ops::DerefMut for IntMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.data.get() }
    }
}
