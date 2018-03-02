//! UIAnimation

use winapi::ctypes::*;
use winapi::shared::ntdef::{ULONG, HRESULT};
use winapi::shared::guiddef::{REFIID, GUID};
use winapi::um::unknwnbase::*;
use std::io::Result as IOResult;
use {ResultCarrier, AsRawHandle};
use std::ptr::null_mut;

pub type Seconds = c_double;
#[repr(C)] pub enum IdleBehavior { Continue = 0, Disable = 1 }
#[repr(C)] pub enum UpdateResult { NoChange = 0, VariablesChanged = 1 }
#[repr(C)] pub enum TimerClientStatus { Idle = 0, Busy = 1 }

#[allow(non_snake_case)]
#[repr(C)] struct IUIAnimationTimerVtbl
{
    pub QueryInterface: extern "system" fn(This: *mut IUIAnimationTimer, riid: REFIID, ppvObject: *mut *mut c_void) -> HRESULT,
    pub AddRef: extern "system" fn(This: *mut IUIAnimationTimer) -> ULONG,
    pub Release: extern "system" fn(This: *mut IUIAnimationTimer) -> ULONG,
    pub SetTimerUpdateHandler: extern "system" fn(This: *mut IUIAnimationTimer, /* opt */ updateHandler: *mut IUIAnimationTimerUpdateHandler, idleBehavior: IdleBehavior) -> HRESULT,
    pub SetTimerEventHandler: extern "system" fn(This: *mut IUIAnimationTimer, /* opt */ handler: *mut IUIAnimationTimerEventHandler) -> HRESULT,
    pub Enable: extern "system" fn(This: *mut IUIAnimationTimer) -> HRESULT,
    pub Disable: extern "system" fn(This: *mut IUIAnimationTimer) -> HRESULT,
    pub IsEnabled: extern "system" fn(This: *mut IUIAnimationTimer) -> HRESULT,
    pub GetTime: extern "system" fn(This: *mut IUIAnimationTimer, seconds: *mut Seconds) -> HRESULT,
    pub SetFrameRateThreshold: extern "system" fn(This: *mut IUIAnimationTimer, framesPerSeconds: u32) -> HRESULT
}
#[repr(C)] pub struct IUIAnimationTimer(*const IUIAnimationTimerVtbl);
#[allow(non_upper_case_globals)]
/// BFCD4A0C-06B6-4384-B768-0DAA792C380E
const CLSID_UIAnimation: GUID = GUID
{
    Data1: 0xbfcd4a0c, Data2: 0x06b6, Data3: 0x4384, Data4: [0xb7, 0x68, 0x0d, 0xaa, 0x79, 0x2c, 0x38, 0x0e]
};
#[allow(non_snake_case)]
impl IUIAnimationTimer
{
    pub unsafe fn AddRef(&mut self) -> ULONG { ((*self.0).AddRef)(self) }
    pub unsafe fn Release(&mut self) -> ULONG { ((*self.0).Release)(self) }
    pub unsafe fn QueryInterface(&mut self, riid: REFIID, ppv_object: *mut *mut c_void) -> HRESULT
    {
        ((*self.0).QueryInterface)(self, riid, ppv_object)
    }
}
/// 6B0EFAD1-A053-41d6-9085-33A689144665
impl ::winapi::Interface for IUIAnimationTimer
{
    fn uuidof() -> GUID
    {
        GUID
        {
            Data1: 0x6b0efad1, Data2: 0xa053, Data3: 0x41d6, Data4: [0x90, 0x85, 0x33, 0xa6, 0x89, 0x14, 0x46, 0x65]
        }
    }
}

pub struct Timer(*mut IUIAnimationTimer); HandleWrapper!(for Timer[IUIAnimationTimer] + FromRawHandle);
impl Timer
{
    pub fn new() -> IOResult<Self>
    {
        ::co_create_inproc_instance(&CLSID_UIAnimation).map(Timer)
    }
    pub fn set_update_handler(&mut self, handler: Option<&AsRawHandle<IUIAnimationTimerUpdateHandler>>,
        idle_behavior: IdleBehavior) -> IOResult<()>
    {
        unsafe
        {
            ((*(*self.0).0).SetTimerUpdateHandler)(self.0,
                handler.map(AsRawHandle::as_raw_handle).unwrap_or(null_mut()), idle_behavior as _).checked()
        }
    }
    pub fn enable(&mut self) -> IOResult<()> { unsafe { ((*(*self.0).0).Enable)(self.0).checked() } }
    pub fn disable(&mut self) -> IOResult<()> { unsafe { ((*(*self.0).0).Disable)(self.0).checked() } }
    pub fn is_enabled(&self) -> IOResult<()> { unsafe { ((*(*self.0).0).IsEnabled)(self.0).checked() } }
    pub fn time(&self) -> IOResult<Seconds>
    {
        let mut secs = 0.0;
        unsafe { ((*(*self.0).0).GetTime)(self.0, &mut secs).to_result(secs) }
    }
    pub fn set_frame_rate_threshold(&mut self, fps: u32) -> IOResult<()>
    {
        unsafe { ((*(*self.0).0).SetFrameRateThreshold)(self.0, fps).checked() }
    }
}

#[allow(non_snake_case)]
#[repr(C)] pub struct IUIAnimationTimerUpdateHandlerVtbl
{
    pub QueryInterface: extern "system" fn(This: *mut IUIAnimationTimerUpdateHandler, riid: REFIID, ppvObject: *mut *mut c_void) -> HRESULT,
    pub AddRef: extern "system" fn(This: *mut IUIAnimationTimerUpdateHandler) -> ULONG,
    pub Release: extern "system" fn(This: *mut IUIAnimationTimerUpdateHandler) -> ULONG,
    pub OnUpdate: extern "system" fn(This: *mut IUIAnimationTimerUpdateHandler, timeNow: Seconds, result: *mut UpdateResult) -> HRESULT,
    pub SetTimerClientEventHandler: extern "system" fn(This: *mut IUIAnimationTimerUpdateHandler, handler: *mut IUIAnimationTimerClientEventHandler) -> HRESULT,
    pub ClearTimerClientEventHandler: extern "system" fn(This: *mut IUIAnimationTimerUpdateHandler) -> HRESULT
}
#[repr(C)] pub struct IUIAnimationTimerUpdateHandler(*const IUIAnimationTimerUpdateHandlerVtbl);
impl IUIAnimationTimerUpdateHandler
{
    #[allow(non_snake_case)]
    pub fn AddRef(&mut self) -> ULONG { unsafe { ((*self.0).AddRef)(self) } }
}

#[allow(non_snake_case)]
#[repr(C)] pub struct IUIAnimationTimerClientEventHandlerVtbl
{
    pub QueryInterface: extern "system" fn(This: *mut IUIAnimationTimerClientEventHandler, riid: REFIID, ppvObject: *mut *mut c_void) -> HRESULT,
    pub AddRef: extern "system" fn(This: *mut IUIAnimationTimerClientEventHandler) -> ULONG,
    pub Release: extern "system" fn(This: *mut IUIAnimationTimerClientEventHandler) -> ULONG,
    pub OnTimerClientStatusChanged: extern "system" fn(This: *mut IUIAnimationTimerClientEventHandler, newStatus: TimerClientStatus, previousStatus: TimerClientStatus) -> HRESULT
}
#[repr(C)] pub struct IUIAnimationTimerClientEventHandler(*const IUIAnimationTimerClientEventHandlerVtbl);

#[allow(non_snake_case)]
#[repr(C)] pub struct IUIAnimationTimerEventHandlerVtbl
{
    pub QueryInterface: extern "system" fn(This: *mut IUIAnimationTimerEventHandler, riid: REFIID, ppvObject: *mut *mut c_void) -> HRESULT,
    pub AddRef: extern "system" fn(This: *mut IUIAnimationTimerEventHandler) -> ULONG,
    pub Release: extern "system" fn(This: *mut IUIAnimationTimerEventHandler) -> ULONG,
    pub OnPreUpdate: extern "system" fn(This: *mut IUIAnimationTimerEventHandler) -> HRESULT,
    pub OnPostUpdate: extern "system" fn(This: *mut IUIAnimationTimerEventHandler) -> HRESULT,
    pub OnRenderingTooSlow: extern "system" fn(This: *mut IUIAnimationTimerEventHandler, framesPerSecond: u32) -> HRESULT
}
#[repr(C)] pub struct IUIAnimationTimerEventHandler(*const IUIAnimationTimerEventHandlerVtbl);
