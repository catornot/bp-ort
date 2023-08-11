use std::{ffi::c_char, marker::PhantomData};

pub struct Pointer<'a, T> {
    pub ptr: *const T,
    marker: PhantomData<&'a T>,
}

impl<'a, T> From<*const T> for Pointer<'a, T> {
    fn from(value: *const T) -> Self {
        Self {
            ptr: value,
            marker: PhantomData,
        }
    }
}

impl<'a, T> From<*mut T> for Pointer<'a, T> {
    fn from(value: *mut T) -> Self {
        Self {
            ptr: value.cast_const(),
            marker: PhantomData,
        }
    }
}

impl<'a, T> From<Pointer<'a, T>> for *const T {
    fn from(val: Pointer<'a, T>) -> Self {
        val.ptr
    }
}

impl<'a, T> From<Pointer<'a, T>> for *mut T {
    fn from(val: Pointer<'a, T>) -> Self {
        val.ptr.cast_mut()
    }
}

#[inline]
pub(crate) unsafe fn iterate_c_array_sized<T, const U: usize>(
    ptr: Pointer<T>,
) -> impl Iterator<Item = &T> {
    let ptr: *const T = ptr.into();
    (0..U).filter_map(move |i| ptr.add(i).as_ref())
}

#[inline]
pub(crate) unsafe fn set_c_char_array<const U: usize>(buf: &mut [c_char; U], new: &str) {
    *buf = [0;U]; // null everything
    buf.iter_mut()
        .zip(new.as_bytes())
        .for_each(|(buf_char, new)| *buf_char = *new as i8);
}
