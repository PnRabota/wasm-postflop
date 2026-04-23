use std::alloc::{alloc_zeroed, dealloc, handle_alloc_error, Layout};
use std::ptr::NonNull;
use std::slice;

#[derive(Debug)]
pub struct AlignedF32Buffer {
    ptr: NonNull<f32>,
    len: usize,
    layout: Layout,
}

unsafe impl Send for AlignedF32Buffer {}
unsafe impl Sync for AlignedF32Buffer {}

impl AlignedF32Buffer {
    pub fn new_zeroed(len: usize, alignment: usize) -> Self {
        assert!(
            alignment.is_power_of_two(),
            "alignment must be power of two"
        );
        assert!(
            alignment >= std::mem::align_of::<f32>(),
            "alignment is smaller than f32 alignment"
        );

        if len == 0 {
            return Self {
                ptr: NonNull::dangling(),
                len,
                layout: Layout::from_size_align(0, alignment).unwrap(),
            };
        }

        let bytes = len
            .checked_mul(std::mem::size_of::<f32>())
            .expect("buffer size overflow");
        let layout = Layout::from_size_align(bytes, alignment).expect("invalid aligned layout");

        let raw_ptr = unsafe { alloc_zeroed(layout) };
        if raw_ptr.is_null() {
            handle_alloc_error(layout);
        }

        Self {
            ptr: NonNull::new(raw_ptr.cast::<f32>()).expect("null pointer after alloc"),
            len,
            layout,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn as_slice(&self) -> &[f32] {
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for AlignedF32Buffer {
    fn drop(&mut self) {
        if self.len == 0 {
            return;
        }
        unsafe {
            dealloc(self.ptr.as_ptr().cast::<u8>(), self.layout);
        }
    }
}
