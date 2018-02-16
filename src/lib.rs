//! COM Driver

extern crate metrics;
extern crate widestring;
extern crate winapi;

use widestring::*;
use std::io::{Result as IOResult, Error as IOError};
use winapi::shared::windef::HWND;
use winapi::shared::winerror::{HRESULT, SUCCEEDED};
use winapi::um::unknwnbase::IUnknown;
use winapi::Interface;
use std::borrow::Cow;

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

use std::ffi::CStr;
use std::ops::Deref;
/// Universal String Trait(convertable to wide string)
pub trait UnivString
{
    /// UTF-16 string
    fn to_wcstr(&self) -> Cow<WideCStr>;
}
impl UnivString for str
{
    fn to_wcstr(&self) -> Cow<WideCStr> { WideCString::from_str(self).unwrap().into() }
}
impl UnivString for String
{
    fn to_wcstr(&self) -> Cow<WideCStr> { WideCString::from_str(self).unwrap().into() }
}
impl UnivString for WideStr
{
    fn to_wcstr(&self) -> Cow<WideCStr> { WideCString::from_wide_str(self).unwrap().into() }
}
impl UnivString for WideString
{
    fn to_wcstr(&self) -> Cow<WideCStr> { WideCString::from_wide_str(self).unwrap().into() }
}
impl UnivString for CStr
{
    fn to_wcstr(&self) -> Cow<WideCStr> { WideCString::from_str(self.to_string_lossy().deref()).unwrap().into() }
}
impl UnivString for WideCStr
{
    fn to_wcstr(&self) -> Cow<WideCStr> { self.into() }
}

/// IUnknownにへんかんできることを保証(AsRawHandle<IUnknown>の特殊化)
pub trait AsIUnknown { fn as_iunknown(&self) -> *mut IUnknown; }
/// 特定のハンドルポインタに変換できることを保証
pub trait AsRawHandle<I: Interface> { fn as_raw_handle(&self) -> *mut I; }
/// 特定のインターフェイスハンドルであり、別インターフェイスをクエリすることができる
pub trait Handle : AsRawHandle<<Self as Handle>::RawType> + AsIUnknown + Drop
{
    type RawType : Interface;
    fn query_interface<Q: Handle>(&self) -> IOResult<Q> where Q: FromRawHandle<<Q as Handle>::RawType>;
}
/// 生のハンドルポインタから構成できる
pub trait FromRawHandle<H> { unsafe fn from_raw_handle(*mut H) -> Self; }
macro_rules! AutoRemover
{
    (for $($t: ty [$ti: ty]),*) =>
    {
        $(impl Drop for $t
        {
            fn drop(&mut self)
            {
                if let Some(p) = unsafe { self.0.as_mut() }
                {
                    /*let rc = */unsafe { p.Release() };
                    // println!("Dropping {}({}) outstanding refcount: {}", stringify!($t), stringify!($ti), rc);
                    self.0 = std::ptr::null_mut();
                }
            }
        })*
    }
}

/// IUnknown Receiver
pub struct Unknown(*mut IUnknown);
AutoRemover!(for Unknown[IUnknown]);
/// Temporary Slot
pub struct ComPtr<T>(pub *mut T);
impl<T> Drop for ComPtr<T>
{
    fn drop(&mut self)
    {
        if let Some(p) = unsafe { self.0.as_mut() }
        {
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

pub mod traits
{
    pub use super::dcomp::{SurfaceFactoryProvider, TargetProvider, SurfaceFactory, Surface};
    pub use super::d2::{RenderTarget, GeometrySegment};
    pub use super::{ResultCarrier, AsIUnknown, AsRawHandle, Handle};
}
pub use self::traits::*;
pub mod submods
{
    pub use super::{d3d, dxgi, d3d11, d3d12, d2, dcomp, dwrite, imaging};
}
