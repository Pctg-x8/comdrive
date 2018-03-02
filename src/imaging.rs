//! Imaging Components Driver

use winapi::um::wincodec::*;
use winapi::um::winnt::GENERIC_READ;
use winapi::um::unknwnbase::LPUNKNOWN;
use winapi::shared::guiddef::{REFIID, REFCLSID, REFGUID};
use winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER;
use winapi::shared::minwindef::{DWORD, LPVOID};
use super::*;
use metrics::*;

// common pixel format guids //
pub use winapi::um::wincodec::GUID_WICPixelFormat32bppPBGRA;

/// Driver object for IWICImagingFactory
pub struct Factory(*mut IWICImagingFactory); HandleWrapper!(for Factory[IWICImagingFactory] + FromRawHandle);
impl Factory
{
    /// Create Instance
    pub fn new() -> IOResult<Self>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, std::ptr::null_mut(), CLSCTX_INPROC_SERVER,
            &IWICImagingFactory::uuidof(), &mut handle).to_result_with(|| Factory(handle as _)) }
    }
}

/// Driver object for IWICBitmapDecoder
pub struct BitmapDecoder(*mut IWICBitmapDecoder); HandleWrapper!(for BitmapDecoder[IWICBitmapDecoder] + FromRawHandle);
impl Factory
{
    /// Create Bitmap Decoder from File
    pub fn new_decoder_from_file<WPath: ::UnivString + ?Sized>(&self, path: &WPath) -> IOResult<BitmapDecoder>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateDecoderFromFilename(path.to_wcstr().as_ptr(), std::ptr::null(), GENERIC_READ,
            WICDecodeMetadataCacheOnDemand, &mut handle).to_result_with(|| BitmapDecoder(handle)) }
    }
}
impl BitmapDecoder
{
    /// Acquire Frame
    pub fn frame(&self, index: usize) -> IOResult<BitmapFrameDecode>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).GetFrame(index as _, &mut handle).to_result_with(|| BitmapFrameDecode(handle)) }
    }
}

/// Driver object for IWICBitmapFrameDecode
pub struct BitmapFrameDecode(*mut IWICBitmapFrameDecode); HandleWrapper!(for BitmapFrameDecode[IWICBitmapFrameDecode] + FromRawHandle);

/// Driver object for IWICFormatConverter
pub struct FormatConverter(*mut IWICFormatConverter); HandleWrapper!(for FormatConverter[IWICFormatConverter] + FromRawHandle);
impl Factory
{
    /// Create Format Converter
    pub fn new_format_converter(&self) -> IOResult<FormatConverter>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateFormatConverter(&mut handle).to_result_with(|| FormatConverter(handle)) }
    }
}
impl FormatConverter
{
    /// Initialize Converter
    pub fn initialize(&self, src: &BitmapFrameDecode, target_format: REFGUID) -> IOResult<()>
    {
        unsafe { (*self.0).Initialize(src.0 as _, target_format, WICBitmapDitherTypeNone,
            std::ptr::null(), 0.0, WICBitmapPaletteTypeMedianCut) }.checked()
    }
    /// Size of bitmap
    pub fn size(&self) -> IOResult<Size2U>
    {
        let (mut w, mut h) = (0, 0);
        unsafe { (*self.0).GetSize(&mut w, &mut h) }.to_result_with(|| Size2U(w, h))
    }
}

#[link(name = "ole32")]
extern "system"
{
    pub(crate) fn CoCreateInstance(rclsid: REFCLSID, pUnkOuter: LPUNKNOWN, dwClsContext: DWORD, riid: REFIID, ppv: *mut LPVOID) -> HRESULT;
}
