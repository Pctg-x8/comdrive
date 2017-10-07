//! Imaging Components Driver

use winapi::um::wincodec::*;
use winapi::um::winnt::GENERIC_READ;
use winapi::um::unknwnbase::LPUNKNOWN;
use winapi::shared::guiddef::{REFIID, REFCLSID, REFGUID};
use winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER;
use winapi::shared::minwindef::{DWORD, LPVOID};
use super::*;
use metrics::*;

/// Driver object for IWICImagingFactory
pub struct Factory(*mut IWICImagingFactory);
impl Handle for Factory
{
    type RawType = IWICImagingFactory;
    fn query_interface<Q: Handle>(&self) -> IOResult<Q> where Q: FromRawHandle<<Q as Handle>::RawType>
    {
        let mut handle: *mut Q::RawType = std::ptr::null_mut();
        unsafe { (*self.0).QueryInterface(&Q::RawType::uuidof(), std::mem::transmute(&mut handle)) }.to_result_with(|| unsafe { Q::from_raw_handle(handle) })
    }
}
impl AsRawHandle<IWICImagingFactory> for Factory { fn as_raw_handle(&self) -> *mut IWICImagingFactory { self.0 } }
impl FromRawHandle<IWICImagingFactory> for Factory { unsafe fn from_raw_handle(h: *mut IWICImagingFactory) -> Self { Factory(h) } }
impl AsIUnknown for Factory { fn as_iunknown(&self) -> *mut IUnknown { self.0 as _ } }
impl Factory
{
    /// Create Instance
    pub fn new() -> IOResult<Self>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, std::ptr::null_mut(), CLSCTX_INPROC_SERVER,
            &IWICImagingFactory::uuidof(), &mut handle) }.to_result_with(|| Factory(handle as _))
    }
}

/// Driver object for IWICBitmapDecoder
pub struct BitmapDecoder(*mut IWICBitmapDecoder);
impl Handle for BitmapDecoder
{
    type RawType = IWICBitmapDecoder;
    fn query_interface<Q: Handle>(&self) -> IOResult<Q> where Q: FromRawHandle<<Q as Handle>::RawType>
    {
        let mut handle: *mut Q::RawType = std::ptr::null_mut();
        unsafe { (*self.0).QueryInterface(&Q::RawType::uuidof(), std::mem::transmute(&mut handle)) }.to_result_with(|| unsafe { Q::from_raw_handle(handle) })
    }
}
impl AsRawHandle<IWICBitmapDecoder> for BitmapDecoder { fn as_raw_handle(&self) -> *mut IWICBitmapDecoder { self.0 } }
impl FromRawHandle<IWICBitmapDecoder> for BitmapDecoder { unsafe fn from_raw_handle(h: *mut IWICBitmapDecoder) -> Self { BitmapDecoder(h) } }
impl AsIUnknown for BitmapDecoder { fn as_iunknown(&self) -> *mut IUnknown { self.0 as _ } }
impl Factory
{
    /// Create Bitmap Decoder from File
    pub fn new_decoder_from_file<WPath: AsRef<WideCStr> + ?Sized>(&self, path: &WPath) -> IOResult<BitmapDecoder>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateDecoderFromFilename(path.as_ref().as_ptr(), std::ptr::null(), GENERIC_READ,
            WICDecodeMetadataCacheOnDemand, &mut handle) }.to_result_with(|| BitmapDecoder(handle))
    }
}
impl BitmapDecoder
{
    /// Acquire Frame
    pub fn frame(&self, index: usize) -> IOResult<BitmapFrameDecode>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).GetFrame(index as _, &mut handle) }.to_result_with(|| BitmapFrameDecode(handle))
    }
}

/// Driver object for IWICBitmapFrameDecode
pub struct BitmapFrameDecode(*mut IWICBitmapFrameDecode);
impl Handle for BitmapFrameDecode
{
    type RawType = IWICBitmapFrameDecode;
    fn query_interface<Q: Handle>(&self) -> IOResult<Q> where Q: FromRawHandle<<Q as Handle>::RawType>
    {
        let mut handle: *mut Q::RawType = std::ptr::null_mut();
        unsafe { (*self.0).QueryInterface(&Q::RawType::uuidof(), std::mem::transmute(&mut handle)) }.to_result_with(|| unsafe { Q::from_raw_handle(handle) })
    }
}
impl AsRawHandle<IWICBitmapFrameDecode> for BitmapFrameDecode { fn as_raw_handle(&self) -> *mut IWICBitmapFrameDecode { self.0 } }
impl FromRawHandle<IWICBitmapFrameDecode> for BitmapFrameDecode { unsafe fn from_raw_handle(h: *mut IWICBitmapFrameDecode) -> Self { BitmapFrameDecode(h) } }
impl AsIUnknown for BitmapFrameDecode { fn as_iunknown(&self) -> *mut IUnknown { self.0 as _ } }

/// Driver object for IWICFormatConverter
pub struct FormatConverter(*mut IWICFormatConverter);
impl Handle for FormatConverter
{
    type RawType = IWICFormatConverter;
    fn query_interface<Q: Handle>(&self) -> IOResult<Q> where Q: FromRawHandle<<Q as Handle>::RawType>
    {
        let mut handle: *mut Q::RawType = std::ptr::null_mut();
        unsafe { (*self.0).QueryInterface(&Q::RawType::uuidof(), std::mem::transmute(&mut handle)) }.to_result_with(|| unsafe { Q::from_raw_handle(handle) })
    }
}
impl AsRawHandle<IWICFormatConverter> for FormatConverter { fn as_raw_handle(&self) -> *mut IWICFormatConverter { self.0 } }
impl FromRawHandle<IWICFormatConverter> for FormatConverter { unsafe fn from_raw_handle(h: *mut IWICFormatConverter) -> Self { FormatConverter(h) } }
impl AsIUnknown for FormatConverter { fn as_iunknown(&self) -> *mut IUnknown { self.0 as _ } }
impl Factory
{
    /// Create Format Converter
    pub fn new_format_converter(&self) -> IOResult<FormatConverter>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateFormatConverter(&mut handle) }.to_result_with(|| FormatConverter(handle))
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

AutoRemover!(for Factory[IWICImagingFactory], BitmapDecoder[IWICBitmapDecoder], BitmapFrameDecode[IWICBitmapFrameDecode], FormatConverter[IWICFormatConverter]);

#[link(name = "ole32")]
extern "system"
{
    fn CoCreateInstance(rclsid: REFCLSID, pUnkOuter: LPUNKNOWN, dwClsContext: DWORD, riid: REFIID, ppv: *mut LPVOID) -> HRESULT;
}
