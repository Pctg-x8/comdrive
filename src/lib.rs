//! COM Driver

use univstring::*;
use std::io::{Result as IOResult, Error as IOError};
use winapi::shared::windef::HWND;
use winapi::shared::ntdef::HRESULT;
use winapi::shared::winerror::SUCCEEDED;
use winapi::um::unknwnbase::IUnknown;
use winapi::Interface;
use winapi::um::unknwnbase::LPUNKNOWN;
use winapi::shared::guiddef::{REFIID, REFCLSID};
use winapi::shared::minwindef::{DWORD, LPVOID};

pub trait ResultCarrier
{
    fn to_result<T>(self, v: T) -> IOResult<T>;
    fn to_result_with<T, F>(self, tf: F) -> IOResult<T> where F: FnOnce() -> T;
    fn checked(self) -> IOResult<()>;
}
impl ResultCarrier for HRESULT
{
    fn to_result<T>(self, v: T) -> IOResult<T>
    {
        if SUCCEEDED(self) { Ok(v) } else { Err(IOError::from_raw_os_error(self)) }
    }
    fn to_result_with<T, F>(self, tf: F) -> IOResult<T> where F: FnOnce() -> T
    {
        if SUCCEEDED(self) { Ok(tf()) } else { Err(IOError::from_raw_os_error(self)) }
    }
    fn checked(self) -> IOResult<()> { if SUCCEEDED(self) { Ok(()) } else { Err(IOError::from_raw_os_error(self)) } }
}

/// IUnknownにへんかんできることを保証(AsRawHandle<IUnknown>の特殊化)
pub trait AsIUnknown { fn as_iunknown(&self) -> *mut IUnknown; }
/// 特定のハンドルポインタに変換できることを保証
pub unsafe trait AsRawHandle<I> { fn as_raw_handle(&self) -> *mut I; }
/// 特定のインターフェイスハンドルであり、別インターフェイスをクエリすることができる
pub trait Handle : AsRawHandle<<Self as Handle>::RawType> + AsIUnknown
{
    type RawType: Interface;
    fn query_interface<Q: Handle>(&self) -> IOResult<Q> where Q: FromRawHandle<<Q as Handle>::RawType>;
}
/// 生のハンドルポインタから構成できる
pub trait FromRawHandle<H> { unsafe fn from_raw_handle(p: *mut H) -> Self; }
macro_rules! AutoRemover {
    (for $($t: ty [$ti: ty]),*) => {
        $(impl Drop for $t {
            fn drop(&mut self) {
                if let Some(p) = unsafe { self.0.as_mut() } {
                    let _rc = unsafe { p.Release() };
                    #[cfg(feature = "trace_releasing")]
                    log::trace!(target: "trace_releasing", "Dropping {}({}@{:x}) outstanding refcount: {}", stringify!($t), stringify!($ti), self.0 as usize, _rc);
                    self.0 = std::ptr::null_mut();
                }
            }
        })*
    }
}
macro_rules! HandleWrapper {
    (for $t: ident[$i: ty]) => {
        impl crate::AsIUnknown for $t { fn as_iunknown(&self) -> *mut crate::IUnknown { self.0 as _ } }
        unsafe impl crate::AsRawHandle<$i> for $t { fn as_raw_handle(&self) -> *mut $i { self.0 } }
        impl crate::Handle for $t {
            type RawType = $i;
            fn query_interface<Q>(&self) -> IOResult<Q> where Q: crate::Handle + crate::FromRawHandle<<Q as crate::Handle>::RawType> {
                let mut handle = std::ptr::null_mut();
                unsafe { (*self.0).QueryInterface(&<Q::RawType as winapi::Interface>::uuidof(), &mut handle).to_result_with(|| Q::from_raw_handle(handle as _)) }
            }
        }
        // Refcounters
        AutoRemover!(for $t[$i]);
    };
    (for $t: ident[$i: ty] + FromRawHandle) => {
        HandleWrapper!(for $t[$i]);
        impl Clone for $t { fn clone(&self) -> Self { unsafe { (*self.0).AddRef() }; $t(self.0) } }
        impl crate::FromRawHandle<$i> for $t { unsafe fn from_raw_handle(p: *mut $i) -> Self { $t(p) } }
    }
}

/// IUnknown Receiver
#[repr(transparent)]
pub struct Unknown(*mut IUnknown);
AutoRemover!(for Unknown[IUnknown]);
/// Temporary Slot
#[repr(transparent)]
pub struct ComPtr<T>(pub *mut T);
impl<T> Drop for ComPtr<T> {
    fn drop(&mut self) {
        if let Some(p) = unsafe { self.0.as_mut() } {
            /*let rc = */unsafe { std::mem::transmute::<_, &mut IUnknown>(p).Release() };
            // println!("Dropping ComPtr outstanding refcount: {}", rc);
            self.0 = std::ptr::null_mut();
        }
    }
}

pub mod d3d;
pub mod dxgi;
pub mod d3d11;
#[macro_use]
pub mod d3d12;
pub mod d3d11on12;
pub mod d2;
pub mod dcomp;
pub mod dwrite;
pub mod imaging;
pub mod uianimation;

pub mod traits
{
    pub use super::dcomp::{SurfaceFactoryProvider, TargetProvider, SurfaceFactory, Surface};
    pub use super::d2::{RenderTarget, GeometrySegment, Shape};
    pub use super::{ResultCarrier, AsIUnknown, AsRawHandle, Handle};
}
pub use self::traits::*;
pub mod submods
{
    pub use super::{d3d, dxgi, d3d11, d3d12, d2, dcomp, dwrite, imaging};
}

use winapi::shared::guiddef::GUID;
/// CoCreateInstance helper(Create InterProcess-Server Object)
pub(crate) fn co_create_inproc_instance<I: Interface>(clsid: &GUID) -> IOResult<*mut I>
{
    use winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER;
    let mut p = std::ptr::null_mut();
    unsafe
    {
        CoCreateInstance(clsid, std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER, &I::uuidof(), &mut p).to_result(p as _)
    }
}

#[link(name = "ole32")]
extern "system"
{
    pub(crate) fn CoCreateInstance(rclsid: REFCLSID, pUnkOuter: LPUNKNOWN, dwClsContext: DWORD, riid: REFIID, ppv: *mut LPVOID) -> HRESULT;
}
