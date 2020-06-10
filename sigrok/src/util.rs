use crate::{Function, SigrokError};
use glib_sys::{
    g_array_free, g_free, g_variant_get_child_value, g_variant_get_fixed_array, g_variant_get_strv,
    g_variant_lookup_value, g_variant_n_children, g_variant_unref, gpointer, GSList, GVariant,
};
use sigrok_sys::{sr_channel_group, sr_dev_driver, sr_dev_inst, sr_dev_options};
use std::borrow::Cow;
use std::convert::TryFrom;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::{size_of, ManuallyDrop};
use std::ops::{Deref, DerefMut};
use std::os::raw::c_char;
use std::ptr::null_mut;
use std::slice;

macro_rules! define_consts {
    ($int:ty, $e:ty, $($variant:ident),+$(,)?) => {
        $(pub const $variant: $int = <$e>::$variant as $int;)+
    };
}

pub unsafe fn c_str<'a>(ptr: *const c_char) -> Cow<'a, str> {
    if ptr.is_null() {
        Cow::Borrowed("")
    } else {
        CStr::from_ptr(ptr).to_string_lossy()
    }
}

pub unsafe fn null_list_count<T>(mut list: *const *const T) -> usize {
    let mut count = 0;
    while !(*list).is_null() {
        count += 1;
        list = list.add(1);
    }
    count
}

pub unsafe fn gslist_iter(mut list: *const GSList) -> impl Iterator<Item = gpointer> {
    std::iter::from_fn(move || {
        if list.is_null() {
            None
        } else {
            let item = (*list).data;
            list = (*list).next;
            Some(item)
        }
    })
}

struct GArray<T> {
    arr: *mut glib_sys::GArray,
    _type: PhantomData<T>,
}

impl<T> Deref for GArray<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { slice::from_raw_parts((*self.arr).data as *const T, (*self.arr).len as usize) }
    }
}

impl<T> Drop for GArray<T> {
    fn drop(&mut self) {
        unsafe {
            g_array_free(self.arr, 1);
        }
    }
}

pub unsafe fn slice_garray<T>(arr: *mut glib_sys::GArray) -> impl Deref<Target = [T]> {
    GArray {
        arr,
        _type: PhantomData,
    }
}

pub unsafe fn get_functions(
    driver: *const sr_dev_driver,
    sdi: *const sr_dev_inst,
    cg: *const sr_channel_group,
) -> Result<Vec<Function>, SigrokError> {
    let arr = sr_dev_options(driver, sdi, cg);
    if arr.is_null() {
        return Err(SigrokError::Err);
    }
    Ok(slice_garray(arr)
        .iter()
        .filter_map(|f: &u32| Function::try_from(*f).ok())
        .collect())
}

pub struct StringArray<'a>(&'a [*const c_char]);

impl<'a> Deref for StringArray<'a> {
    type Target = [*const c_char];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> Drop for StringArray<'a> {
    fn drop(&mut self) {
        unsafe {
            g_free(self.0.as_ptr() as *mut _);
        }
    }
}

pub struct Variant(pub *mut GVariant);

impl Variant {
    pub unsafe fn get_fixed_array<T>(&self) -> Option<&[T]> {
        let mut num_elements = 0;
        let ptr = g_variant_get_fixed_array(self.0, &mut num_elements, size_of::<T>()) as *const T;
        if ptr.is_null() {
            return None;
        }
        Some(slice::from_raw_parts(ptr, num_elements))
    }

    pub unsafe fn get_str_array(&self) -> Option<StringArray> {
        let mut num_elements = 0;
        let ptr = g_variant_get_strv(self.0, &mut num_elements);
        if ptr.is_null() {
            return None;
        }
        Some(StringArray(slice::from_raw_parts(ptr, num_elements)))
    }

    pub unsafe fn num_children(&self) -> usize {
        g_variant_n_children(self.0)
    }

    pub unsafe fn get_child_value(&self, i: usize) -> Option<Self> {
        let child = g_variant_get_child_value(self.0, i);
        if child.is_null() {
            None
        } else {
            Some(Variant(child))
        }
    }

    pub unsafe fn lookup_value(&self, key: &[u8], value_type: &[u8]) -> Option<Self> {
        let child = g_variant_lookup_value(
            self.0,
            key.as_ptr() as *const _,
            value_type.as_ptr() as *const _,
        );
        if child.is_null() {
            None
        } else {
            Some(Variant(child))
        }
    }
}

impl Drop for Variant {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                g_variant_unref(self.0);
            }
        }
    }
}

pub struct AutoDrop<T>(ManuallyDrop<*mut T>, unsafe fn(*mut T));

impl<T> AutoDrop<T> {
    pub unsafe fn new(val: *mut T, drop: unsafe fn(*mut T)) -> Result<AutoDrop<T>, SigrokError> {
        if val.is_null() {
            Err(SigrokError::Arg)
        } else {
            Ok(AutoDrop(ManuallyDrop::new(val), drop))
        }
    }

    pub unsafe fn null() -> AutoDrop<T> {
        AutoDrop(ManuallyDrop::new(null_mut()), |_| {})
    }
}

impl<T> Deref for AutoDrop<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &**(self.0) }
    }
}

impl<T> DerefMut for AutoDrop<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut **(self.0) }
    }
}

impl<T> Drop for AutoDrop<T> {
    fn drop(&mut self) {
        unsafe { (self.1)(ManuallyDrop::take(&mut self.0)) }
    }
}

pub mod raw_error_code {
    define_consts!(
        i32,
        sigrok_sys::sr_error_code,
        SR_OK,
        SR_ERR,
        SR_ERR_MALLOC,
        SR_ERR_ARG,
        SR_ERR_BUG,
        SR_ERR_SAMPLERATE,
        SR_ERR_NA,
        SR_ERR_DEV_CLOSED,
        SR_ERR_TIMEOUT,
        SR_ERR_CHANNEL_GROUP,
        SR_ERR_DATA,
        SR_ERR_IO
    );
}
