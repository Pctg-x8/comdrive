//! DirectWrite Driver

use winapi::um::dwrite::*;
use winapi::um::dwrite_1::*;
use super::*;
use metrics::*;
use winapi::ctypes::c_void;

pub use winapi::um::dwrite::DWRITE_TEXT_METRICS as TextMetrics;
#[repr(C)] #[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum FontStyle
{
    None = DWRITE_FONT_STYLE_NORMAL as _, Oblique = DWRITE_FONT_STYLE_OBLIQUE as _, Italic = DWRITE_FONT_STYLE_ITALIC as _
}

/// Driver class for IDWriteFactory
pub struct Factory(*mut IDWriteFactory); HandleWrapper!(for Factory[IDWriteFactory]);
impl FromRawHandle<IDWriteFactory> for Factory { unsafe fn from_raw_handle(h: *mut IDWriteFactory) -> Self { Factory(h) } }
impl Factory
{
    /// Create
    pub fn new() -> IOResult<Self>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IDWriteFactory::uuidof(), &mut handle).to_result_with(|| Factory(handle as _)) }
    }
}

pub struct FontOptions
{
    pub weight: DWRITE_FONT_WEIGHT, pub style: FontStyle, pub stretch: DWRITE_FONT_STRETCH
}
impl Default for FontOptions
{
    fn default() -> Self
    {
        FontOptions { weight: DWRITE_FONT_WEIGHT_NORMAL, style: FontStyle::None, stretch: DWRITE_FONT_STRETCH_NORMAL }
    }
}

/// Driver object for IDWriteTextFormat
pub struct TextFormat(*mut IDWriteTextFormat); HandleWrapper!(for TextFormat[IDWriteTextFormat]);
impl FromRawHandle<IDWriteTextFormat> for TextFormat { unsafe fn from_raw_handle(h: *mut IDWriteTextFormat) -> Self { TextFormat(h) } }
impl Factory
{
    /// Create Text Format
    pub fn new_text_format<Name: ::UnivString + ?Sized>(&self, family_name: &Name, collection: Option<&FontCollection>, size: f32, options: FontOptions) -> IOResult<TextFormat>
    {
        let ws_ja_jp = WideCString::from_str("ja-JP").unwrap();
        let mut handle = std::ptr::null_mut();
        unsafe
        {
            (*self.0).CreateTextFormat(family_name.to_wcstr().as_ptr(), collection.as_ref().map(|x| x.0).unwrap_or(std::ptr::null_mut()),
                options.weight, options.style as _, options.stretch, size, ws_ja_jp.as_ptr(), &mut handle).to_result_with(|| TextFormat(handle))
        }
    }
}

/// Driver object for IDWriteTextLayout1
pub struct TextLayout(*mut IDWriteTextLayout1); HandleWrapper!(for TextLayout[IDWriteTextLayout1]);
impl FromRawHandle<IDWriteTextLayout1> for TextLayout { unsafe fn from_raw_handle(h: *mut IDWriteTextLayout1) -> Self { TextLayout(h) } }
impl Factory
{
    /// Create Text Layout
    pub fn new_text_layout<Content: ::UnivString + ?Sized>(&self, content: &Content, format: &TextFormat, max_width: f32, max_height: f32)
        -> IOResult<TextLayout>
    {
        let mut handle = std::ptr::null_mut();
        let content_w = content.to_wcstr();
        unsafe { (*self.0).CreateTextLayout(content_w.as_ptr(), content_w.len() as _, format.0, max_width, max_height, &mut handle) }
            .to_result(handle).and_then(|h| unsafe
            {
                let mut handle1 = std::ptr::null_mut();
                (*h).QueryInterface(&IDWriteTextLayout1::uuidof(), &mut handle1).to_result_with(||
                {
                    (*h).Release();
                    TextLayout(handle1 as _)
                })
            })
    }
}
impl TextLayout
{
    /// Metrics of this layout
    pub fn metrics(&self) -> IOResult<TextMetrics>
    {
        unsafe
        {
            let mut metr = std::mem::uninitialized();
            (*self.0).GetMetrics(&mut metr).to_result(metr)
        }
    }
    /// Size Metrics of this layout
    pub fn size(&self) -> IOResult<Size2F>
    {
        self.metrics().map(|m| Size2F(m.width, m.height))
    }
    /// set character spacing
    pub fn set_character_spacing(&self, space: f32) -> IOResult<()>
    {
        unsafe
        {
            (*self.0).SetCharacterSpacing(space / 2.0, space / 2.0, space, DWRITE_TEXT_RANGE { startPosition: 0, length: std::u32::MAX }).checked()
        }
    }
}

/// フォントコレクション
pub struct FontCollection(*mut IDWriteFontCollection); HandleWrapper!(for FontCollection[IDWriteFontCollection]);
impl Factory
{
    pub fn system_font_collection(&self, check_for_updates: bool) -> IOResult<FontCollection>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).GetSystemFontCollection(&mut handle, check_for_updates as _).to_result_with(|| FontCollection(handle)) }
    }
    /// フォントコレクションローダ(各自で実装)を登録
    pub fn register_font_collection_loader(&self, loader: *mut IDWriteFontCollectionLoader) -> IOResult<()>
    {
        unsafe { (*self.0).RegisterFontCollectionLoader(loader).checked() }
    }
    /// カスタムフォントコレクションを作成
    pub fn new_custom_font_collection<KeyT>(&self, loader: *mut IDWriteFontCollectionLoader, key: KeyT) -> IOResult<FontCollection>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateCustomFontCollection(loader, &key as *const KeyT as *const c_void, std::mem::size_of::<KeyT>() as _, &mut handle).to_result_with(|| FontCollection(handle)) }
    }
    /// フォントコレクションローダの削除
    pub fn unregister_font_collection_loader(&self, loader: *mut IDWriteFontCollectionLoader) -> IOResult<()>
    {
        unsafe { (*self.0).UnregisterFontCollectionLoader(loader).checked() }
    }
}

/// フォントファイル
pub struct FontFile(*mut IDWriteFontFile); HandleWrapper!(for FontFile[IDWriteFontFile]);
impl Factory
{
    pub fn new_font_file_reference<WPath: ::UnivString + ?Sized>(&self, path: &WPath) -> IOResult<FontFile>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateFontFileReference(path.to_wcstr().as_ptr(), std::ptr::null(), &mut handle).to_result_with(|| FontFile(handle)) }
    }
}
