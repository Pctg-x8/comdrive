//! DXGI Device

use winapi::ctypes::c_void;
use winapi::shared::minwindef::UINT;
use winapi::shared::dxgi::*;
use winapi::shared::dxgi1_2::*;
use winapi::shared::dxgi1_3::DXGI_CREATE_FACTORY_DEBUG;
use winapi::shared::dxgi1_4::*;
use winapi::shared::dxgitype::*;
use winapi::um::libloaderapi::{LoadLibraryA, FreeLibrary, GetProcAddress};
use winapi::shared::minwindef::{ULONG};
use winapi::shared::guiddef::{GUID, REFIID};
use super::*;
use metrics::{Size, Size2U};

pub use winapi::shared::dxgitype::DXGI_SAMPLE_DESC as SampleDesc;
pub use winapi::shared::dxgiformat::DXGI_FORMAT as Format;
#[repr(C)] #[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum AlphaMode
{
    Unspecified = DXGI_ALPHA_MODE_UNSPECIFIED as _, Premultiplied = DXGI_ALPHA_MODE_PREMULTIPLIED as _,
    Straight = DXGI_ALPHA_MODE_STRAIGHT as _, Ignored = DXGI_ALPHA_MODE_IGNORE as _
}

/// most used formats
pub use winapi::shared::dxgiformat::
{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R32_FLOAT,
    DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32G32B32_FLOAT, DXGI_FORMAT_R32G32B32A32_FLOAT
};

/// Driver object for IDXGIFactory2
#[repr(transparent)]
pub struct Factory(*mut IDXGIFactory2);
impl Factory
{
    /// Create
    pub fn new(debug: bool) -> IOResult<Self>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { CreateDXGIFactory2(if debug { DXGI_CREATE_FACTORY_DEBUG } else { 0 }, &IDXGIFactory2::uuidof(), &mut handle) }
            .to_result_with(|| Factory(handle as _))
    }
    pub fn adapter(&self, index: usize) -> IOResult<Adapter>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).EnumAdapters(index as UINT, &mut handle) }.to_result_with(|| Adapter(handle as _))
    }
}
/// Driver object for IDXGIAdapter
#[repr(transparent)]
pub struct Adapter(*mut IDXGIAdapter); HandleWrapper!(for Adapter[IDXGIAdapter] + FromRawHandle);
/// Driver object for IDXGIDevice
#[repr(transparent)]
pub struct Device(*mut IDXGIDevice1); HandleWrapper!(for Device[IDXGIDevice1] + FromRawHandle);
/// Driver object for IDXGISurface
#[repr(transparent)]
pub struct Surface(*mut IDXGISurface); HandleWrapper!(for Surface[IDXGISurface] + FromRawHandle);

pub trait DeviceChild { fn parent(&self) -> IOResult<Device>; }
impl DeviceChild for Device { fn parent(&self) -> IOResult<Device> { Ok(self.clone()) } }
pub trait SurfaceChild { fn base(&self) -> IOResult<Surface>; }
impl SurfaceChild for Surface { fn base(&self) -> IOResult<Surface> { Ok(self.clone()) } }
impl Device
{
    pub fn adapter(&self) -> IOResult<Adapter>
    {
        let mut a = std::ptr::null_mut();
        unsafe { (*self.0).GetAdapter(&mut a) }.to_result_with(|| Adapter(a))
    }
    pub fn set_maximum_frame_latency(&self, max_latency: u32) -> IOResult<()>
    {
        unsafe { (*self.0).SetMaximumFrameLatency(max_latency) }.checked()
    }
}
impl Adapter
{
    pub fn parent<H: Handle>(&self) -> IOResult<H> where H: FromRawHandle<<H as Handle>::RawType>
    {
        let mut h = std::ptr::null_mut();
        unsafe { (*self.0).GetParent(&<H as Handle>::RawType::uuidof(), &mut h) }
            .to_result_with(|| unsafe { H::from_raw_handle(h as _) })
    }
    pub fn desc(&self) -> IOResult<DXGI_ADAPTER_DESC>
    {
        let mut s = std::mem::MaybeUninit::uninit();
        unsafe { (*self.0).GetDesc(s.as_mut_ptr()).to_result(s.assume_init()) }
    }
}

#[allow(non_camel_case_types)] pub type DXGI_DEBUG_RLO_FLAGS = u8;
#[allow(dead_code)] const DXGI_DEBUG_RLO_SUMMARY: DXGI_DEBUG_RLO_FLAGS = 0x01;
#[allow(dead_code)] const DXGI_DEBUG_RLO_DETAIL: DXGI_DEBUG_RLO_FLAGS = 0x02;
#[allow(dead_code)] const DXGI_DEBUG_RLO_IGNORE_INTERNAL: DXGI_DEBUG_RLO_FLAGS = 0x04;
const DXGI_DEBUG_RLO_ALL: DXGI_DEBUG_RLO_FLAGS = 0x07;
#[allow(non_snake_case, dead_code)] #[repr(C)]
pub struct IDXGIDebugVtbl
{
    QueryInterface: unsafe extern "system" fn(*mut IDXGIDebug, REFIID, *mut *mut c_void) -> HRESULT,
    AddRef: unsafe extern "system" fn(*mut IDXGIDebug) -> ULONG,
    Release: unsafe extern "system" fn(*mut IDXGIDebug) -> ULONG,

    ReportLiveObjects: unsafe extern "system" fn(*mut IDXGIDebug, GUID, DXGI_DEBUG_RLO_FLAGS) -> HRESULT
}
#[repr(transparent)]
pub struct IDXGIDebug(*const IDXGIDebugVtbl);
impl winapi::Interface for IDXGIDebug
{
    fn uuidof() -> GUID
    {
        GUID { Data1: 0x119e7452, Data2: 0xde9e, Data3: 0x40fe, Data4: [0x88, 0x06, 0x88, 0xf9, 0x0c, 0x12, 0xb4, 0x41] }
    }
}
#[allow(non_snake_case)]
impl IDXGIDebug
{
    pub unsafe fn QueryInterface(&mut self, riid: REFIID, ppvObject: *mut *mut c_void) -> HRESULT
    {
        ((*self.0).QueryInterface)(self, riid, ppvObject)
    }
    pub unsafe fn AddRef(&mut self) -> ULONG { ((*self.0).AddRef)(self) }
    pub unsafe fn Release(&mut self) -> ULONG { ((*self.0).Release)(self) }
}
const DEBUG_ALL: GUID = GUID { Data1: 0xe48ae283, Data2: 0xda80, Data3: 0x490b, Data4: [0x87, 0xe6, 0x43, 0xe9, 0xa9, 0xcf, 0xda, 0x08] };
const DEBUG_DX: GUID = GUID { Data1: 0x35cdd7fc, Data2: 0x13b2, Data3: 0x421d, Data4: [0xa5, 0xd7, 0x7e, 0x44, 0x51, 0x28, 0x7d, 0x64] };
const DEBUG_DXGI: GUID = GUID { Data1: 0x25cddaa4, Data2: 0xb1c6, Data3: 0x47e1, Data4: [0xac, 0x3e, 0x98, 0x87, 0x5b, 0x5a, 0x2e, 0x2a] };
const DEBUG_APP: GUID = GUID { Data1: 0x6cd6e01, Data2: 0x4219, Data3: 0x4ebd, Data4: [0x87, 0x09, 0x27, 0xed, 0x23, 0x36, 0x0c, 0x62] };
pub enum DebugRegion { All, DirectX, DXGI, App }
/// デバッグインターフェイス
#[repr(transparent)]
pub struct Debug(*mut IDXGIDebug); HandleWrapper!(for Debug[IDXGIDebug] + FromRawHandle);
impl Debug
{
    pub fn get() -> IOResult<Self>
    {
        let lib = unsafe { LoadLibraryA("dxgidebug.dll\x00".as_ptr() as *const _) };
        if lib.is_null() { return Err(IOError::last_os_error()); };
        let dxgi_get_debug_interface: unsafe extern "system" fn(REFIID, *mut *mut c_void) -> HRESULT = unsafe
        {
            std::mem::transmute(GetProcAddress(lib, "DXGIGetDebugInterface\x00".as_ptr() as *const _))
        };
        let mut handle = std::ptr::null_mut();
        let handle = unsafe
        {
            (dxgi_get_debug_interface)(&IDXGIDebug::uuidof(), &mut handle).to_result_with(|| Debug(handle as _))?
        };
        unsafe { FreeLibrary(lib) }; Ok(handle)
    }
    pub fn report_live_objects(&self, region: DebugRegion) -> IOResult<()>
    {
        unsafe { ((*(*self.0).0).ReportLiveObjects)(self.0, match region
        {
            DebugRegion::All => DEBUG_ALL, DebugRegion::DirectX => DEBUG_DX, DebugRegion::DXGI => DEBUG_DXGI, DebugRegion::App => DEBUG_APP
        }, DXGI_DEBUG_RLO_ALL) }.checked()
    }
}

/// スワップチェーン
pub struct SwapChain(*mut IDXGISwapChain3, Format, usize); HandleWrapper!(for SwapChain[IDXGISwapChain3]);
impl Factory
{
    /// スワップチェーンの作成
    pub fn new_swapchain<RenderDevice: AsIUnknown>(&self, rendering_device: &RenderDevice, init_size: Size2U,
        format: Format, alpha_mode: AlphaMode, buffer_count: usize, use_sequential: bool) -> IOResult<SwapChain>
    {
        let desc = DXGI_SWAP_CHAIN_DESC1
        {
            BufferCount: buffer_count as _, BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            Format: format, AlphaMode: alpha_mode as _, Width: init_size.width(), Height: init_size.height(),
            Stereo: false as _, SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            SwapEffect: if use_sequential { DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL } else { DXGI_SWAP_EFFECT_FLIP_DISCARD },
            Scaling: DXGI_SCALING_STRETCH, Flags: 0
        };
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateSwapChainForComposition(rendering_device.as_iunknown(), &desc, std::ptr::null_mut(), &mut handle) }
            .to_result(handle as *mut IDXGISwapChain1).and_then(|h|
            {
                let mut h3 = std::ptr::null_mut();
                unsafe { (*h).QueryInterface(&IDXGISwapChain3::uuidof(), &mut h3) }.to_result_with(|| unsafe
                {
                    (*h).Release(); SwapChain(h3 as _, format, buffer_count)
                })
            })
    }
    pub fn new_swapchain_for_hwnd<RenderDevice: AsIUnknown>(&self, render: &RenderDevice, target: HWND, init_size: Size2U,
        format: Format, alpha_mode: AlphaMode, buffer_count: usize, use_sequential: bool) -> IOResult<SwapChain>
    {
        let desc = DXGI_SWAP_CHAIN_DESC1
        {
            BufferCount: buffer_count as _, BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            Format: format, AlphaMode: alpha_mode as _, Width: init_size.width(), Height: init_size.height(),
            Stereo: false as _, SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            SwapEffect: if use_sequential { DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL } else { DXGI_SWAP_EFFECT_FLIP_DISCARD },
            Scaling: DXGI_SCALING_STRETCH, Flags: 0
        };
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateSwapChainForHwnd(render.as_iunknown(), target, &desc, std::ptr::null(), std::ptr::null_mut(), &mut handle) }
            .to_result(handle as *mut IDXGISwapChain1).and_then(|h|
            {
                let mut h3 = std::ptr::null_mut();
                unsafe { (*h).QueryInterface(&IDXGISwapChain3::uuidof(), &mut h3) }.to_result_with(|| unsafe
                {
                    (*h).Release(); SwapChain(h3 as _, format, buffer_count)
                })
            })
    }
}
impl SwapChain
{
    /// バックバッファリソースを取得
    pub fn back_buffer<Surface: Handle>(&self, index: usize) -> IOResult<Surface>
        where Surface: FromRawHandle<<Surface as Handle>::RawType>
    {
        let mut s = std::ptr::null_mut();
        unsafe { (*self.0).GetBuffer(index as _, &Surface::RawType::uuidof(), &mut s) }.to_result_with(|| unsafe { Surface::from_raw_handle(s as _) })
    }
    /// リサイズ
    pub fn resize(&self, new_size: Size2U) -> IOResult<()>
    {
        unsafe { (*self.0).ResizeBuffers(self.2 as _, new_size.width(), new_size.height(), self.1, 0) }.checked()
    }
    /// 現在のバックバッファインデックスを取得
    pub fn current_back_buffer_index(&self) -> u32 { unsafe { (*self.0).GetCurrentBackBufferIndex() } }
    /// 表示
    pub fn present(&self) -> IOResult<()> { unsafe { (*self.0).Present(0, 0) }.checked() }
}

#[link(name = "dxgi")]
extern "system"
{
    fn CreateDXGIFactory2(Flags: UINT, riid: REFIID, ppFactory: *mut *mut c_void) -> HRESULT;
}
