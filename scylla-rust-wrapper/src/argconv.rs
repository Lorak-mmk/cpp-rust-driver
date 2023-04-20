use crate::types::size_t;
use std::cmp::min;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;

pub unsafe fn ptr_to_cstr(ptr: *const c_char) -> Option<&'static str> {
    CStr::from_ptr(ptr).to_str().ok()
}

pub unsafe fn ptr_to_cstr_n(ptr: *const c_char, size: size_t) -> Option<&'static str> {
    std::str::from_utf8(std::slice::from_raw_parts(ptr as *const u8, size as usize)).ok()
}

pub unsafe fn arr_to_cstr<const N: usize>(arr: &[c_char]) -> Option<&'static str> {
    let null_char = '\0' as c_char;
    let end_index = arr[..N].iter().position(|c| c == &null_char).unwrap_or(N);
    ptr_to_cstr_n(arr.as_ptr(), end_index as size_t)
}

pub fn str_to_arr<const N: usize>(s: &str) -> [c_char; N] {
    let mut result = ['\0' as c_char; N];

    // Max length must be null-terminated
    let mut max_len = min(N - 1, s.as_bytes().len());

    while !s.is_char_boundary(max_len) {
        max_len -= 1;
    }

    for (i, c) in s.as_bytes().iter().enumerate().take(max_len) {
        result[i] = *c as c_char;
    }

    result
}

pub unsafe fn write_str_to_c(s: &str, c_str: *mut *const c_char, c_strlen: *mut size_t) {
    *c_str = s.as_ptr() as *const c_char;
    *c_strlen = s.len() as u64;
}

pub unsafe fn strlen(ptr: *const c_char) -> size_t {
    if ptr.is_null() {
        return 0;
    }
    libc::strlen(ptr) as size_t
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct CassConstPtr<T: ?Sized> {
    ptr: Option<NonNull<T>>,
}

impl<T: ?Sized> CassConstPtr<T> {
    fn new(ptr: *const T) -> Self {
        CassConstPtr {
            ptr: NonNull::new(ptr as *mut T),
        }
    }

    // Guarantee: for Some the pointer inside is non-null
    fn into_raw_ptr(self) -> Option<*const T> {
        self.ptr.map(|p| p.as_ptr() as *const T)
    }

    pub fn null() -> Self {
        CassConstPtr { ptr: None }
    }

    pub fn is_null(self) -> bool {
        self.ptr.is_none()
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct CassMutPtr<T: ?Sized> {
    ptr: Option<NonNull<T>>,
}

impl<T: ?Sized> CassMutPtr<T> {
    fn new(ptr: *mut T) -> Self {
        CassMutPtr {
            ptr: NonNull::new(ptr),
        }
    }

    // Guarantee: for Some the pointer inside is non-null
    fn into_raw_ptr(self) -> Option<*mut T> {
        self.ptr.map(|p| p.as_ptr())
    }

    pub fn into_const(self) -> CassConstPtr<T> {
        CassConstPtr { ptr: self.ptr }
    }

    pub fn null_mut() -> Self {
        CassMutPtr { ptr: None }
    }

    pub fn is_null(self) -> bool {
        self.ptr.is_none()
    }
}

pub trait BoxFFI {
    fn into_ptr(data: Box<Self>) -> CassMutPtr<Self> {
        #[allow(clippy::disallowed_methods)]
        CassMutPtr::new(Box::into_raw(data))
    }
    unsafe fn from_ptr(ptr: CassMutPtr<Self>) -> Option<Box<Self>> {
        #[allow(clippy::disallowed_methods)]
        ptr.into_raw_ptr().map(|p| Box::from_raw(p))
    }
    unsafe fn as_ref<'a>(ptr: CassConstPtr<Self>) -> Option<&'a Self> {
        #[allow(clippy::disallowed_methods)]
        // SAFETY: CassConstPtr::into_raw_ptr guarantees the pointer is not null
        ptr.into_raw_ptr().map(|p| p.as_ref().unwrap_unchecked())
    }
    unsafe fn as_mut_ref<'a>(ptr: CassMutPtr<Self>) -> Option<&'a mut Self> {
        #[allow(clippy::disallowed_methods)]
        // SAFETY: CassConstPtr::into_raw_ptr guarantees the pointer is not null
        ptr.into_raw_ptr().map(|p| p.as_mut().unwrap_unchecked())
    }
    unsafe fn free(ptr: CassMutPtr<Self>) {
        std::mem::drop(BoxFFI::from_ptr(ptr));
    }
}

pub trait ArcFFI {
    fn as_ptr(data: &Arc<Self>) -> CassConstPtr<Self> {
        #[allow(clippy::disallowed_methods)]
        CassConstPtr::new(Arc::as_ptr(data))
    }
    fn into_ptr(data: Arc<Self>) -> CassConstPtr<Self> {
        #[allow(clippy::disallowed_methods)]
        CassConstPtr::new(Arc::into_raw(data))
    }
    unsafe fn from_ptr(ptr: CassConstPtr<Self>) -> Option<Arc<Self>> {
        #[allow(clippy::disallowed_methods)]
        ptr.into_raw_ptr().map(|p| Arc::from_raw(p))
    }
    unsafe fn cloned_from_ptr(ptr: CassConstPtr<Self>) -> Option<Arc<Self>> {
        let ptr = ptr.into_raw_ptr()?;
        #[allow(clippy::disallowed_methods)]
        Arc::increment_strong_count(ptr);
        #[allow(clippy::disallowed_methods)]
        Some(Arc::from_raw(ptr))
    }
    unsafe fn as_ref<'a>(ptr: CassConstPtr<Self>) -> Option<&'a Self> {
        #[allow(clippy::disallowed_methods)]
        // SAFETY: CassConstPtr::into_raw_ptr guarantees the pointer is not null
        ptr.into_raw_ptr().map(|p| p.as_ref().unwrap_unchecked())
    }
    unsafe fn free(ptr: CassConstPtr<Self>) {
        std::mem::drop(ArcFFI::from_ptr(ptr));
    }
}

pub trait RcFFI {
    fn as_ptr(data: &Rc<Self>) -> CassConstPtr<Self> {
        #[allow(clippy::disallowed_methods)]
        CassConstPtr::new(Rc::as_ptr(data))
    }
    fn into_ptr(data: Rc<Self>) -> CassConstPtr<Self> {
        #[allow(clippy::disallowed_methods)]
        CassConstPtr::new(Rc::into_raw(data))
    }
    unsafe fn from_ptr(ptr: CassConstPtr<Self>) -> Option<Rc<Self>> {
        #[allow(clippy::disallowed_methods)]
        ptr.into_raw_ptr().map(|p| Rc::from_raw(p))
    }
    unsafe fn cloned_from_ptr(ptr: CassConstPtr<Self>) -> Option<Rc<Self>> {
        let ptr = ptr.into_raw_ptr()?;
        #[allow(clippy::disallowed_methods)]
        Rc::increment_strong_count(ptr);
        #[allow(clippy::disallowed_methods)]
        Some(Rc::from_raw(ptr))
    }
    unsafe fn as_ref<'a>(ptr: CassConstPtr<Self>) -> Option<&'a Self> {
        #[allow(clippy::disallowed_methods)]
        // SAFETY: CassConstPtr::into_raw_ptr guarantees the pointer is not null
        ptr.into_raw_ptr().map(|p| p.as_ref().unwrap_unchecked())
    }
    unsafe fn free(ptr: CassConstPtr<Self>) {
        std::mem::drop(RcFFI::from_ptr(ptr));
    }
}

pub trait RefFFI {
    fn as_ptr(&self) -> CassConstPtr<Self> {
        CassConstPtr::new(self as *const Self)
    }
    unsafe fn as_ref<'a>(ptr: CassConstPtr<Self>) -> Option<&'a Self> {
        #[allow(clippy::disallowed_methods)]
        // SAFETY: CassConstPtr::into_raw_ptr guarantees the pointer is not null
        ptr.into_raw_ptr().map(|p| p.as_ref().unwrap_unchecked())
    }
    unsafe fn as_mut_ref<'a>(ptr: CassMutPtr<Self>) -> Option<&'a mut Self> {
        #[allow(clippy::disallowed_methods)]
        // SAFETY: CassConstPtr::into_raw_ptr guarantees the pointer is not null
        ptr.into_raw_ptr().map(|p| p.as_mut().unwrap_unchecked())
    }
}
