//! DirectWrite Driver

use winapi::um::dwrite::*;
use winapi::um::dwrite_1::*;
use winapi::shared::minwindef::FLOAT;
use super::*;
use metrics::*;
use winapi::ctypes::c_void;
pub use winapi::um::dwrite::DWRITE_GLYPH_OFFSET as GlyphOffset;
use std::ops::Deref;
use std::mem::{size_of, MaybeUninit};
use std::ptr::{null_mut, null};
use std::slice;

pub use winapi::um::dwrite::{
    DWRITE_TEXT_METRICS as TextMetrics, DWRITE_FONT_METRICS as FontMetrics, DWRITE_LINE_METRICS as LineMetrics,
    DWRITE_OVERHANG_METRICS as OverhangMetrics, DWRITE_GLYPH_METRICS as GlyphMetrics,
    DWRITE_FONT_FACE_TYPE as FontFaceType, DWRITE_FONT_FILE_TYPE as FontFileType
};
pub use winapi::um::dwrite::{
    DWRITE_FONT_WEIGHT as FontWeight,
    DWRITE_FONT_WEIGHT_THIN as FONT_WEIGHT_THIN, DWRITE_FONT_WEIGHT_EXTRA_LIGHT as FONT_WEIGHT_EXTRA_LIGHT,
    DWRITE_FONT_WEIGHT_ULTRA_LIGHT as FONT_WEIGHT_ULTRA_LIGHT, DWRITE_FONT_WEIGHT_LIGHT as FONT_WEIGHT_LIGHT,
    DWRITE_FONT_WEIGHT_NORMAL as FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_REGULAR as FONT_WEIGHT_REGULAR,
    DWRITE_FONT_WEIGHT_MEDIUM as FONT_WEIGHT_MEDIUM, DWRITE_FONT_WEIGHT_DEMI_BOLD as FONT_WEIGHT_DEMI_BOLD,
    DWRITE_FONT_WEIGHT_SEMI_BOLD as FONT_WEIGHT_SEMI_BOLD, DWRITE_FONT_WEIGHT_BOLD as FONT_WEIGHT_BOLD,
    DWRITE_FONT_WEIGHT_EXTRA_BOLD as FONT_WEIGHT_EXTRA_BOLD, DWRITE_FONT_WEIGHT_ULTRA_BOLD as FONT_WEIGHT_ULTRA_BOLD,
    DWRITE_FONT_WEIGHT_BLACK as FONT_WEIGHT_BLACK, DWRITE_FONT_WEIGHT_HEAVY as FONT_WEIGHT_HEAVY,
    DWRITE_FONT_WEIGHT_EXTRA_BLACK as FONT_WEIGHT_EXTRA_BLACK, DWRITE_FONT_WEIGHT_ULTRA_BLACK as FONT_WEIGHT_ULTRA_BLACK
};
pub use winapi::um::dwrite::{
    DWRITE_FONT_STRETCH as FontStretch,
    DWRITE_FONT_STRETCH_ULTRA_CONDENSED as FONT_STRETCH_ULTRA_CONDENSED,
    DWRITE_FONT_STRETCH_EXTRA_CONDENSED as FONT_STRETCH_EXTRA_CONDENSED,
    DWRITE_FONT_STRETCH_CONDENSED as FONT_STRETCH_CONDENSED,
    DWRITE_FONT_STRETCH_SEMI_CONDENSED as FONT_STRECH_SEMI_CONDENSED,
    DWRITE_FONT_STRETCH_NORMAL as FONT_STRETCH_NORMAL, DWRITE_FONT_STRETCH_MEDIUM as FONT_STRETCH_MEDIUM,
    DWRITE_FONT_STRETCH_SEMI_EXPANDED as FONT_STRETCH_SEMI_EXPANDED,
    DWRITE_FONT_STRETCH_EXPANDED as FONT_STRETCH_EXPANDED,
    DWRITE_FONT_STRETCH_EXTRA_EXPANDED as FONT_STRETCH_EXTRA_EXPANDED,
    DWRITE_FONT_STRETCH_ULTRA_EXPANDED as FONT_STRETCH_ULTRA_EXPANDED
};
pub use winapi::um::dwrite::{
    DWRITE_FONT_STYLE as FontStyle,
    DWRITE_FONT_STYLE_NORMAL as FONT_STYLE_NORMAL,
    DWRITE_FONT_STYLE_OBLIQUE as FONT_STYLE_OBLIQUE,
    DWRITE_FONT_STYLE_ITALIC as FONT_STYLE_ITALIC
};
pub use winapi::um::dwrite::{
    DWRITE_FONT_SIMULATIONS as FontSimulations,
    DWRITE_FONT_SIMULATIONS_NONE as FONT_SIMULATIONS_NONE,
    DWRITE_FONT_SIMULATIONS_BOLD as FONT_SIMULATIONS_BOLD,
    DWRITE_FONT_SIMULATIONS_OBLIQUE as FONT_SIMULATIONS_OBLIQUE
};

/// Driver class for IDWriteFactory
#[repr(transparent)]
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

    pub fn register_font_file_loader<Loader: AsRawHandle<IDWriteFontFileLoader>>(&self, loader: &Loader) -> IOResult<()> {
        unsafe { (*self.0).RegisterFontFileLoader(loader.as_raw_handle()).to_result(()) }
    }
    pub fn unregister_font_file_loader<Loader: AsRawHandle<IDWriteFontFileLoader>>(&self, loader: &Loader) -> IOResult<()> {
        unsafe { (*self.0).UnregisterFontFileLoader(loader.as_raw_handle()).to_result(()) }
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
        FontOptions { weight: FONT_WEIGHT_NORMAL, style: FONT_STYLE_NORMAL, stretch: FONT_STRETCH_NORMAL }
    }
}

/// Driver object for IDWriteTextFormat
#[repr(transparent)]
pub struct TextFormat(*mut IDWriteTextFormat); HandleWrapper!(for TextFormat[IDWriteTextFormat]);
impl FromRawHandle<IDWriteTextFormat> for TextFormat { unsafe fn from_raw_handle(h: *mut IDWriteTextFormat) -> Self { TextFormat(h) } }
impl Factory
{
    /// Create Text Format
    pub fn new_text_format<Name: UnivString + ?Sized>(&self, family_name: &Name, collection: Option<&FontCollection>, size: f32, options: FontOptions) -> IOResult<TextFormat>
    {
        let ws_ja_jp = "ja-JP".to_wcstr().unwrap();
        let fam = family_name.to_wcstr().unwrap();
        let mut handle = std::ptr::null_mut();
        unsafe
        {
            (*self.0).CreateTextFormat(fam.as_ptr(), collection.as_ref().map(|x| x.0).unwrap_or(std::ptr::null_mut()),
                options.weight, options.style as _, options.stretch, size, ws_ja_jp.as_ptr(), &mut handle).to_result_with(|| TextFormat(handle))
        }
    }
}
impl TextFormat {
    pub fn font_collection(&self) -> IOResult<FontCollection> {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).GetFontCollection(&mut handle).to_result_with(|| FontCollection(handle)) }
    }
    pub fn font_family_name(&self) -> IOResult<widestring::WideCString> {
        let length = unsafe { (*self.0).GetFontFamilyNameLength() };
        let mut family_name = vec![0; 1 + length as usize];
        unsafe {
            (*self.0).GetFontFamilyName(family_name.as_mut_ptr(), family_name.len() as _)
                .to_result_with(|| widestring::WideCString::from_vec_with_nul_unchecked(family_name))
        }
    }

    pub fn weight(&self) -> FontWeight {
        unsafe { (*self.0).GetFontWeight() }
    }
    pub fn stretch(&self) -> FontStretch {
        unsafe { (*self.0).GetFontStretch() }
    }
    pub fn style(&self) -> FontStyle {
        unsafe { (*self.0).GetFontStyle() }
    }
}

/// Driver object for IDWriteTextLayout1
#[repr(transparent)]
pub struct TextLayout(*mut IDWriteTextLayout1); HandleWrapper!(for TextLayout[IDWriteTextLayout1]);
impl FromRawHandle<IDWriteTextLayout1> for TextLayout { unsafe fn from_raw_handle(h: *mut IDWriteTextLayout1) -> Self { TextLayout(h) } }
impl Factory
{
    /// Create Text Layout
    pub fn new_text_layout<Content: UnivString + ?Sized>(&self, content: &Content, format: &TextFormat, max_width: f32, max_height: f32)
        -> IOResult<TextLayout>
    {
        let mut handle = std::ptr::null_mut();
        let content_w = content.to_wcstr().unwrap();
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
            let mut metr = MaybeUninit::uninit();
            (*self.0).GetMetrics(metr.as_mut_ptr()).to_result(metr.assume_init())
        }
    }
    /// Overhanging Metrics of the Layout
    pub fn overhang_metrics(&self) -> IOResult<OverhangMetrics>
    {
        unsafe
        {
            let mut metr = MaybeUninit::uninit();
            (*self.0).GetOverhangMetrics(metr.as_mut_ptr()).to_result(metr.assume_init())
        }
    }
    /// Metrics of each lines
    pub fn line_metrics(&self) -> IOResult<Vec<LineMetrics>>
    {
        unsafe
        {
            let mut count = 0;
            (*self.0).GetLineMetrics(null_mut(), 0, &mut count);
            let mut metrics = Vec::with_capacity(count as _); metrics.set_len(count as _);
            (*self.0).GetLineMetrics(metrics.as_mut_ptr(), count, &mut count).to_result(metrics)
        }
    }
    /// Size Metrics of this layout
    pub fn size(&self) -> IOResult<Size2F>
    {
        self.metrics().map(|m| Size2F(m.width, m.height))
    }
    /// set character spacing
    pub fn set_character_spacing(&self, space_pre: f32, space_post: f32, min_advance: f32) -> IOResult<()>
    {
        unsafe
        {
            (*self.0).SetCharacterSpacing(space_pre, space_post, min_advance,
                DWRITE_TEXT_RANGE { startPosition: 0, length: std::u32::MAX }).checked()
        }
    }
    /// Drawing the layout by calling back to the renderer object.
    pub fn draw<R: AsRawHandle<IDWriteTextRenderer>>(&self, callback: &R, context: *mut c_void, origin: &Point2F) -> IOResult<()>
    {
        unsafe { (*self.0).Draw(context, callback.as_raw_handle(), origin.x(), origin.y()).checked() }
    }
    /// Set underline for character range
    pub fn set_underline(&self, has_underline: bool, range: std::ops::Range<u32>) -> IOResult<()> {
        unsafe {
            (*self.0).SetUnderline(
                has_underline as _, DWRITE_TEXT_RANGE { startPosition: range.start, length: range.end - range.start }
            ).checked()
        }
    }
}

/// フォントファミリー
#[repr(transparent)]
pub struct FontFamily(*mut IDWriteFontFamily); HandleWrapper!(for FontFamily[IDWriteFontFamily]);
impl FontCollection
{
    pub fn find_family_name<S: UnivString + ?Sized>(&self, name: &S) -> IOResult<Option<u32>>
    {
        let (mut index, mut exists) = (0, 0);
        let n = name.to_wcstr().unwrap();
        unsafe
        {
            (*self.0).FindFamilyName(n.as_ptr(), &mut index, &mut exists)
                .to_result_with(|| if exists == 0 { None } else { Some(index) })
        }
    }
    pub fn font_family(&self, index: u32) -> IOResult<FontFamily>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).GetFontFamily(index, &mut handle).to_result_with(|| FontFamily(handle)) }
    }
}
impl FontFamily
{
    pub fn first_matching_font(&self, weight: FontWeight, stretch: FontStretch, style: FontStyle) -> IOResult<Font>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).GetFirstMatchingFont(weight, stretch, style as _, &mut handle).to_result_with(|| Font(handle)) }
    }
}
impl Deref for FontFamily
{
    type Target = FontList; fn deref(&self) -> &FontList { unsafe { std::mem::transmute(self) } }
}

/// フォントリスト
#[repr(transparent)]
pub struct FontList(*mut IDWriteFontList); HandleWrapper!(for FontList[IDWriteFontList]);

/// フォント
#[repr(transparent)]
pub struct Font(*mut IDWriteFont); HandleWrapper!(for Font[IDWriteFont]);
impl FontList {
    pub fn font(&self, index: u32) -> IOResult<Font> {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).GetFont(index, &mut handle).to_result_with(|| Font(handle)) }
    }
}
impl Font {
    pub fn metrics(&self) -> FontMetrics {
        let mut metr = MaybeUninit::uninit();
        unsafe { (*self.0).GetMetrics(metr.as_mut_ptr()) };
        unsafe { metr.assume_init() }
    }
}

/// フォントフェイス
#[repr(transparent)]
pub struct FontFace(*mut IDWriteFontFace); HandleWrapper!(for FontFace[IDWriteFontFace]);
impl Font
{
    pub fn new_font_face(&self) -> IOResult<FontFace>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateFontFace(&mut handle).to_result_with(|| FontFace(handle)) }
    }
}
impl Factory {
    pub fn new_font_face(
        &self,
        face_type: FontFaceType,
        files: &[*mut IDWriteFontFile],
        face_index: u32,
        simulation_flags: FontSimulations
    ) -> IOResult<FontFace> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.0).CreateFontFace(
                face_type, files.len() as _, files.as_ptr(), face_index, simulation_flags, &mut handle
            ).to_result_with(|| FontFace(handle))
        }
    }
}
impl FontFace
{
    pub fn metrics(&self) -> FontMetrics
    {
        let mut metr = MaybeUninit::uninit();
        unsafe
        {
            (*self.0).GetMetrics(metr.as_mut_ptr());
            metr.assume_init()
        }
    }
    pub fn glyph_indices(&self, codepoints: &[char]) -> IOResult<Vec<u16>>
    {
        let mut indices = Vec::with_capacity(codepoints.len()); unsafe { indices.set_len(codepoints.len()); }
        unsafe
        {
            (*self.0).GetGlyphIndices(codepoints.as_ptr() as _, codepoints.len() as _, indices.as_mut_ptr())
                .to_result(indices)
        }
    }
    /// グリフ列のアウトラインを計算し、指定されたシンクオブジェクトにコールバックする
    pub fn sink_glyph_run_outline<S: AsRawHandle<IDWriteGeometrySink>>(
        &self, emsize: FLOAT, indices: &[u16],
        advances: Option<&[FLOAT]>, offsets: Option<&[GlyphOffset]>,
        sideways: bool, rtl: bool, sink: &mut S) -> IOResult<()>
    {
        assert!(advances.as_ref().map_or(true, |v| v.len() == indices.len()), "Mismatched a number of advances");
        assert!(offsets.as_ref().map_or(true, |v| v.len() == indices.len()), "Mismatched a number of offsets");
        unsafe
        {
            (*self.0).GetGlyphRunOutline(emsize, indices.as_ptr(),
                advances.map(|x| x.as_ptr()).unwrap_or(null()),
                offsets.map(|x| x.as_ptr()).unwrap_or(null()),
                indices.len() as _, sideways as _, rtl as _, sink.as_raw_handle()).checked()
        }
    }
    /// フォントファイルの列挙
    pub fn files(&self) -> IOResult<Vec<FontFile>>
    {
        let mut num = 0;
        unsafe { (*self.0).GetFiles(&mut num, null_mut()).checked()?; }
        let mut vf = Vec::with_capacity(num as _); unsafe { vf.set_len(num as _); }
        unsafe { (*self.0).GetFiles(&mut num, vf.as_mut_ptr() as *mut _).to_result(vf) }
    }

    /// グリフ単位のメトリックを取得
    pub fn design_glyph_metrics(&self, glyphs: &[u16], is_sideways: bool) -> IOResult<Vec<GlyphMetrics>>
    {
        let mut sink = Vec::with_capacity(glyphs.len());
        unsafe { sink.set_len(glyphs.len()); }

        unsafe
        {
            (*self.0).GetDesignGlyphMetrics(glyphs.as_ptr() as _, glyphs.len() as _, sink.as_mut_ptr(),
                is_sideways as _).to_result(sink)
        }
    }
}

/// フォントコレクション
#[repr(transparent)]
pub struct FontCollection(*mut IDWriteFontCollection); HandleWrapper!(for FontCollection[IDWriteFontCollection]);
impl Factory
{
    pub fn system_font_collection(&self, check_for_updates: bool) -> IOResult<FontCollection>
    {
        let mut handle = null_mut();
        unsafe
        {
            (*self.0).GetSystemFontCollection(&mut handle, check_for_updates as _)
                .to_result_with(|| FontCollection(handle))
        }
    }
    /// フォントコレクションローダ(各自で実装)を登録
    pub fn register_font_collection_loader(&self, loader: *mut IDWriteFontCollectionLoader) -> IOResult<()>
    {
        unsafe { (*self.0).RegisterFontCollectionLoader(loader).checked() }
    }
    /// カスタムフォントコレクションを作成
    pub fn new_custom_font_collection<KeyT>(&self, loader: *mut IDWriteFontCollectionLoader, key: KeyT) -> IOResult<FontCollection>
    {
        let mut handle = null_mut();
        unsafe
        {
            (*self.0).CreateCustomFontCollection(loader, &key as *const _ as *const c_void, size_of::<KeyT>() as _, &mut handle)
                .to_result_with(|| FontCollection(handle))
        }
    }
    /// フォントコレクションローダの削除
    pub fn unregister_font_collection_loader(&self, loader: *mut IDWriteFontCollectionLoader) -> IOResult<()>
    {
        unsafe { (*self.0).UnregisterFontCollectionLoader(loader).checked() }
    }
}

/// フォントファイル
#[repr(transparent)]
pub struct FontFile(*mut IDWriteFontFile); HandleWrapper!(for FontFile[IDWriteFontFile]);
impl Factory
{
    pub fn new_font_file_reference<WPath: UnivString + ?Sized>(&self, path: &WPath) -> IOResult<FontFile>
    {
        let mut handle = null_mut();
        let p = path.to_wcstr().unwrap();
        unsafe
        {
            (*self.0).CreateFontFileReference(p.as_ptr(), std::ptr::null(), &mut handle)
                .to_result_with(|| FontFile(handle))
        }
    }
    pub fn new_custom_font_file_reference<Loader: AsRawHandle<IDWriteFontFileLoader>, T>(
        &self, reference_key: &T, loader: &Loader
    ) -> IOResult<FontFile> {
        let mut handle = null_mut();
        unsafe {
            (*self.0).CreateCustomFontFileReference(
                reference_key as *const T as _, std::mem::size_of::<T>() as _, loader.as_raw_handle(), &mut handle
            ).to_result_with(|| FontFile(handle))
        }
    }
}
impl FontFile
{
    /// ローダにおけるこのファイルへの参照キー
    pub fn reference_key(&self) -> IOResult<&[u8]>
    {
        let (mut ptr, mut size) = (null(), 0);
        unsafe
        {
            (*self.0).GetReferenceKey(&mut ptr as *mut _ as *mut *const _, &mut size)
                .to_result_with(|| slice::from_raw_parts(ptr, size as _))
        }
    }

    /// (isSupportedFontType, fontFileType, fontFaceType, numberOfFaces)
    pub fn analyze(&self) -> IOResult<(bool, FontFileType, FontFaceType, u32)> {
        let (mut is_supported, mut file_type, mut face_type, mut face_count) = (0, 0, 0, 0);

        unsafe {
            (*self.0).Analyze(&mut is_supported, &mut file_type, &mut face_type, &mut face_count)
                .to_result_with(|| (is_supported != 0, file_type, face_type, face_count))
        }
    }
}

/// フォントファイルローダ
#[repr(transparent)]
pub struct FontFileLoader(*mut IDWriteFontFileLoader); HandleWrapper!(for FontFileLoader[IDWriteFontFileLoader]);
impl FontFile
{
    /// 関連付けられたファイルローダ
    pub fn loader(&self) -> IOResult<FontFileLoader>
    {
        let mut handle = null_mut();
        unsafe { (*self.0).GetLoader(&mut handle).to_result_with(|| FontFileLoader(handle)) }
    }
}

/// フォントファイルストリーム
#[repr(transparent)]
pub struct FontFileStream(*mut IDWriteFontFileStream); HandleWrapper!(for FontFileStream[IDWriteFontFileStream]);
impl FontFileLoader
{
    pub fn new_stream_from_key(&self, refkey: &[u8]) -> IOResult<FontFileStream>
    {
        let mut handle = null_mut();
        unsafe
        {
            (*self.0).CreateStreamFromKey(refkey.as_ptr() as *const _, refkey.len() as _, &mut handle)
                .to_result_with(|| FontFileStream(handle))
        }
    }
}
impl FontFileStream
{
    pub fn file_size(&self) -> IOResult<usize>
    {
        let mut size = 0;
        unsafe { (*self.0).GetFileSize(&mut size).to_result(size as _) }
    }
}

/// フォントファイルの断片
pub struct FontFileFragment<'a> { pub data_ptr: &'a [u8], release: *mut c_void, owner: &'a FontFileStream }
impl FontFileStream
{
    pub fn read_fragment(&self, offset: usize, size: usize) -> IOResult<FontFileFragment>
    {
        let (mut dptr, mut ctx) = (null(), null_mut());
        unsafe
        {
            (*self.0).ReadFileFragment(&mut dptr as *mut _ as *mut *const _, offset as _, size as _, &mut ctx)
                .to_result_with(|| FontFileFragment
                {
                    data_ptr: slice::from_raw_parts(dptr, size), release: ctx, owner: self
                })
        }
    }
}
impl<'a> Drop for FontFileFragment<'a>
{
    fn drop(&mut self) { unsafe { (*self.owner.0).ReleaseFileFragment(self.release); } }
}
