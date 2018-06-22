//! Direct2D Driver

use super::*;
use winapi::um::d2d1::*;
use winapi::um::d2d1_1::*;
use winapi::um::d2d1effects::*;
use winapi::um::dcommon::*;
use winapi::shared::dxgiformat::DXGI_FORMAT_UNKNOWN;
use std::ptr::{null, null_mut};
use metrics::*;
use std::borrow::Borrow;

pub use winapi::um::d2d1::{D2D1_COLOR_F as ColorF, D2D1_SIZE_F as SizeF, D2D1_ELLIPSE as Ellipse};
#[repr(C)] #[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntialiasMode
{
    Aliased = D2D1_ANTIALIAS_MODE_ALIASED as _, PerPrimitive = D2D1_ANTIALIAS_MODE_PER_PRIMITIVE as _
}

/// Driver object for ID2D1Factory
pub struct Factory(*mut ID2D1Factory); HandleWrapper!(for Factory[ID2D1Factory] + FromRawHandle);
impl Factory
{
    /// Create
    pub fn new(mt: bool) -> IOResult<Self>
    {
        let mut handle = std::ptr::null_mut();
        unsafe
        {
            D2D1CreateFactory(if mt { D2D1_FACTORY_TYPE_MULTI_THREADED } else { D2D1_FACTORY_TYPE_SINGLE_THREADED },
                &ID2D1Factory::uuidof(), std::ptr::null(), &mut handle)
        }.to_result_with(|| Factory(handle as _))
    }
}

/// Driver object for ID2D1Device
pub struct Device(*mut ID2D1Device); HandleWrapper!(for Device[ID2D1Device] + FromRawHandle);
impl Device
{
    /// Create on Direct3D Device
    pub fn new<DC: dxgi::DeviceChild>(dev3: &DC, mt: bool) -> IOResult<Self>
    {
        let cp = D2D1_CREATION_PROPERTIES
        {
            debugLevel: D2D1_DEBUG_LEVEL_WARNING,
            threadingMode: if mt { D2D1_THREADING_MODE_MULTI_THREADED } else { D2D1_THREADING_MODE_SINGLE_THREADED },
            options: D2D1_DEVICE_CONTEXT_OPTIONS_NONE
        };
        let mut handle = std::ptr::null_mut();
        unsafe { D2D1CreateDevice(dev3.parent()?.as_raw_handle() as _, &cp, &mut handle).to_result_with(|| Device(handle)) }
    }
    pub fn factory(&self) -> Factory
    {
        let mut p = std::ptr::null_mut();
        unsafe { (*self.0).GetFactory(&mut p); } Factory(p)
    }
}

/// Transparent Color
pub const TRANSPARENT_COLOR: ColorF = ColorF { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

/// Driver object for ID2D1HwndRenderTarget
pub struct HwndRenderTarget(*mut ID2D1HwndRenderTarget); HandleWrapper!(for HwndRenderTarget[ID2D1HwndRenderTarget] + FromRawHandle);
impl Factory
{
    pub fn new_hwnd_render_target(&self, target: HWND) -> IOResult<HwndRenderTarget>
    {
        let rtprops = D2D1_RENDER_TARGET_PROPERTIES
        {
            _type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_UNKNOWN, alphaMode: D2D1_ALPHA_MODE_UNKNOWN },
            dpiX: 0.0, dpiY: 0.0, usage: D2D1_RENDER_TARGET_USAGE_NONE,
            minLevel: D2D1_FEATURE_LEVEL_DEFAULT
        };
        let hwrtprops = D2D1_HWND_RENDER_TARGET_PROPERTIES
        {
            hwnd: target, pixelSize: D2D1_SIZE_U { width: 0, height: 0 },
            presentOptions: D2D1_PRESENT_OPTIONS_NONE
        };
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateHwndRenderTarget(&rtprops, &hwrtprops, &mut handle).to_result_with(|| HwndRenderTarget(handle)) }
    }
}
impl HwndRenderTarget
{
    pub fn resize<S: Borrow<D2D1_SIZE_U> + ?Sized>(&self, new_size: &S) -> IOResult<()>
    {
        unsafe { (*self.0).Resize(new_size.borrow()).checked() }
    }
}

/// Driver object for ID2D1DeviceContext
pub struct DeviceContext(*mut ID2D1DeviceContext); HandleWrapper!(for DeviceContext[ID2D1DeviceContext] + FromRawHandle);
impl Device
{
    pub fn new_context(&self, enable_mt_optimizations: bool) -> IOResult<DeviceContext>
    {
        let mut handle = std::ptr::null_mut();
        let opts = if enable_mt_optimizations { D2D1_DEVICE_CONTEXT_OPTIONS_ENABLE_MULTITHREADED_OPTIMIZATIONS } else { 0 };
        unsafe { (*self.0).CreateDeviceContext(opts, &mut handle).to_result_with(|| DeviceContext(handle)) }
    }
}

use winapi::um::d2d1::D2D1_SIZE_F;

/// RenderTarget系の共通実装
pub trait RenderTarget
{
    /// コンテキストハンドル
    fn as_rt_handle(&self) -> *mut ID2D1RenderTarget;

    /// 描画開始
    fn begin_draw(&self) -> &Self { unsafe { (*self.as_rt_handle()).BeginDraw() }; self }
    /// 描画終了
    fn end_draw(&self) -> IOResult<()> { unsafe { (*self.as_rt_handle()).EndDraw(null_mut(), null_mut()).checked() } }
    /// クリップ範囲の設定
    fn push_aa_clip<Rect: Borrow<D2D1_RECT_F> + ?Sized>(&self, rect: &Rect, aliasing: AntialiasMode) -> &Self
    {
        unsafe { (*self.as_rt_handle()).PushAxisAlignedClip(rect.borrow(), aliasing as _) }; self
    }
    /// クリップ範囲を解除
    fn pop_aa_clip(&self) -> &Self { unsafe { (*self.as_rt_handle()).PopAxisAlignedClip() }; self }
    
    /// トランスフォーム行列をセット
    fn set_transform<Matrix: Borrow<D2D1_MATRIX_3X2_F> + ?Sized>(&self, matrix: &Matrix) -> &Self
    {
        unsafe { (*self.as_rt_handle()).SetTransform(matrix.borrow()) }; self
    }
    /// 描画ターゲットの中身を消去
    fn clear<C: Borrow<ColorF> + ?Sized>(&self, color: &C) -> &Self { unsafe { (*self.as_rt_handle()).Clear(color.borrow()) }; self }

    /// 矩形を塗りつぶし
    #[deprecated = "use overrided version: fill<Rect2F>(...)"]
    fn fill_rect<B: Brush + ?Sized, R: Borrow<D2D1_RECT_F> + ?Sized>(&self, area: &R, brush: &B) -> &Self
    {
        unsafe { (*self.as_rt_handle()).FillRectangle(area.borrow(), brush.as_raw_brush()) }; self
    }
    /// 矩形枠線
    #[deprecated = "use overrided version: draw<Rect2F>(...)"]
    fn draw_rect<B: Brush + ?Sized, R: Borrow<D2D1_RECT_F> + ?Sized>(&self, area: &R, brush: &B) -> &Self
    {
        unsafe { (*self.as_rt_handle()).DrawRectangle(area.borrow(), brush.as_raw_brush(), 1.0, null_mut()) }; self
    }
    /// 楕円
    #[deprecated = "use overrided version: fill<Ellipse>(...)"]
    fn fill_ellipse<B: Brush + ?Sized>(&self, shape: &Ellipse, brush: &B) -> &Self
    {
        unsafe { (*self.as_rt_handle()).FillEllipse(shape, brush.as_raw_brush()); } self
    }
    /// 任意の形状
    fn draw<S: Shape + ?Sized, B: Brush + ?Sized>(&self, shape: &S, brush: &B, line_width: f32) -> &Self
    {
        unsafe { shape.draw(&mut *self.as_rt_handle(), brush, line_width); } self
    }
    /// 任意の形状 塗りつぶし
    fn fill<S: Shape + ?Sized, B: Brush + ?Sized>(&self, shape: &S, brush: &B) -> &Self
    {
        unsafe { shape.fill(&mut *self.as_rt_handle(), brush); } self
    }
    /// 線を引く
    #[deprecated = "use overrided version: draw<Point2F .. Point2F>(...)"]
    fn draw_line<B: Brush + ?Sized, P1, P2>(&self, start: &P1, end: &P2, brush: &B, line_width: f32) -> &Self
        where P1: Borrow<D2D1_POINT_2F> + ?Sized, P2: Borrow<D2D1_POINT_2F> + ?Sized
    {
        unsafe { (*self.as_rt_handle()).DrawLine(*start.borrow(), *end.borrow(), brush.as_raw_brush(), line_width, null_mut()) };
        self
    }
    /// レイアウト済みテキストの描画
    fn draw_text<B: Brush + ?Sized, P: Borrow<D2D1_POINT_2F> + ?Sized>(&self, p: &P, layout: &dwrite::TextLayout, brush: &B) -> &Self
    {
        unsafe { (*self.as_rt_handle()).DrawTextLayout(*p.borrow(), layout.as_raw_handle() as _, brush.as_raw_brush(), D2D1_DRAW_TEXT_OPTIONS_NONE) };
        self
    }
    /// テキストの描画
    fn draw_raw_text<S: UnivString + ?Sized, B: Brush + ?Sized, R>(&self, r: &R, text: &S, format: &dwrite::TextFormat, brush: &B) -> &Self
        where R: Borrow<D2D1_RECT_F> + ?Sized
    {
        let tw = text.to_wcstr().unwrap();
        unsafe
        {
            (*self.as_rt_handle()).DrawText(tw.as_ptr(), tw.len() as _, format.as_raw_handle(), r.borrow(), brush.as_raw_brush(), D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL)
        };
        self
    }

    /// ビットマップを描く
    fn draw_bitmap<R: Borrow<D2D1_RECT_F> + ?Sized>(&self, bmp: &Bitmap, rect: &R) -> &Self
    {
        unsafe { (*self.as_rt_handle()).DrawBitmap(bmp.0, rect.borrow(), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, null()) };
        self
    }

    /// ブラシの作成
    fn new_solid_color_brush<C: Borrow<D2D1_COLOR_F> + ?Sized>(&self, col: &C) -> IOResult<SolidColorBrush>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.as_rt_handle()).CreateSolidColorBrush(col.borrow(), std::ptr::null(), &mut handle) }.to_result_with(|| SolidColorBrush(handle))
    }
    /// Create Linear Gradient Brush
    fn new_linear_gradient_brush<P1, P2>(&self, from: &P1, to: &P2, stops: &GradientStopCollection) -> IOResult<LinearGradientBrush>
        where P1: Borrow<D2D1_POINT_2F> + ?Sized, P2: Borrow<D2D1_POINT_2F> + ?Sized
    {
        let mut handle = std::ptr::null_mut();
        let lb_props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: *from.borrow(), endPoint: *to.borrow() };
        let brush_props = D2D1_BRUSH_PROPERTIES { opacity: 1.0, transform: Matrix3x2F::identity().unwrap() };
        unsafe { (*self.as_rt_handle()).CreateLinearGradientBrush(&lb_props, &brush_props, stops.0, &mut handle).to_result_with(|| LinearGradientBrush(handle)) }
    }
    /// Create Radial Gradient Brush
    fn new_radial_gradient_brush<P, S>(&self, center: &P, radius: &S, stops: &GradientStopCollection) -> IOResult<RadialGradientBrush>
        where P: Borrow<D2D1_POINT_2F> + ?Sized, S: Borrow<D2D1_SIZE_F> + ?Sized
    {
        let mut handle = std::ptr::null_mut();
        let rb_props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES
        {
            center: *center.borrow(), radiusX: radius.borrow().width, radiusY: radius.borrow().height,
            gradientOriginOffset: D2D1_POINT_2F { x: 0.0, y: 0.0 }
        };
        let brush_props = D2D1_BRUSH_PROPERTIES { opacity: 1.0, transform: Matrix3x2F::identity().unwrap() };
        unsafe { (*self.as_rt_handle()).CreateRadialGradientBrush(&rb_props, &brush_props, stops.0, &mut handle).to_result_with(|| RadialGradientBrush(handle)) }
    }
    /// Create Gradient Stop Collection
    fn new_gradient_stop_collection(&self, stops: &[GradientStop], gamma: Gamma, extend_mode: ExtendMode) -> IOResult<GradientStopCollection>
    {
        let mut handle = std::ptr::null_mut();
        unsafe
        {
            (*self.as_rt_handle()).CreateGradientStopCollection(stops.as_ptr() as *const _, stops.len() as _, gamma as _, extend_mode as _, &mut handle)
                .to_result_with(|| GradientStopCollection(handle))
        }
    }
}

/// 形状から描画方式を自動推定
pub trait Shape
{
    fn draw<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B, line_width: f32);
    fn fill<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B);
}
impl Shape for Rect2F
{
    fn draw<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B, line_width: f32) { unsafe { p_rt.DrawRectangle(self.borrow(), brush.as_raw_brush(), line_width, null_mut()); } }
    fn fill<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B) { unsafe { p_rt.FillRectangle(self.borrow(), brush.as_raw_brush()); } }
}
impl Shape for Ellipse
{
    fn draw<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B, line_width: f32) { unsafe { p_rt.DrawEllipse(self, brush.as_raw_brush(), line_width, null_mut()); } }
    fn fill<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B) { unsafe { p_rt.FillEllipse(self, brush.as_raw_brush()); } }
}
/// 線(start .. end)
impl<P: Borrow<D2D1_POINT_2F>> Shape for ::std::ops::Range<P>
{
    fn draw<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B, line_width: f32)
    {
        unsafe { p_rt.DrawLine(*self.start.borrow(), *self.end.borrow(), brush.as_raw_brush(), line_width, null_mut()); }
    }
    fn fill<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B) { self.draw(p_rt, brush, 1.0) }
}

/// 垂線
pub struct VLine { pub x: f32, pub top: f32, pub bottom: f32 }
/// 水平線
pub struct HLine { pub y: f32, pub left: f32, pub right: f32 }
impl Shape for VLine
{
    fn draw<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B, line_width: f32)
    {
        unsafe { p_rt.DrawLine(Point2F { x: self.x, y: self.top }, Point2F { x: self.x, y: self.bottom }, brush.as_raw_brush(), line_width, null_mut()) };
    }
    fn fill<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B) { self.draw(p_rt, brush, 1.0); }
}
impl Shape for HLine
{
    fn draw<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B, line_width: f32)
    {
        unsafe { p_rt.DrawLine(Point2F { x: self.left, y: self.y }, Point2F { x: self.right, y: self.y }, brush.as_raw_brush(), line_width, null_mut()) };
    }
    fn fill<B: Brush + ?Sized>(&self, p_rt: &mut ID2D1RenderTarget, brush: &B) { self.draw(p_rt, brush, 1.0); }
}

impl RenderTarget for HwndRenderTarget { fn as_rt_handle(&self) -> *mut ID2D1RenderTarget { self.0 as _ } }
impl RenderTarget for DeviceContext { fn as_rt_handle(&self) -> *mut ID2D1RenderTarget { self.0 as _ } }
impl DeviceContext
{
    /// Imageを描く
    pub fn draw<IMG: Image + ?Sized, P: Borrow<D2D1_POINT_2F> + ?Sized>(&self, offs: &P, image: &IMG) -> &Self
    {
        unsafe { (*self.0).DrawImage(image.as_raw_image(), offs.borrow(), std::ptr::null(), D2D1_INTERPOLATION_MODE_LINEAR, D2D1_COMPOSITE_MODE_SOURCE_OVER) };
        self
    }
    /// Effectを描く
    pub fn draw_effected<E: Effect + ?Sized, P: Borrow<D2D1_POINT_2F> + ?Sized>(&self, offs: &P, fx: &E) -> &Self { self.draw(offs, &fx.get_output()) }
}
/// Driver object for ID2D1Bitmap(Context bound object)
pub struct Bitmap(*mut ID2D1Bitmap); HandleWrapper!(for Bitmap[ID2D1Bitmap] + FromRawHandle);
impl DeviceContext
{
    /// Receive Converted Pixels
    pub fn new_bitmap_from_converter(&self, conv: &imaging::FormatConverter) -> IOResult<Bitmap>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateBitmapFromWicBitmap(conv.as_raw_handle() as _, std::ptr::null(), &mut handle) }
            .to_result_with(|| Bitmap(handle as _))
    }
}
pub enum RenderableBitmapSource<'s>
{
    FromDxgiSurface(&'s dxgi::SurfaceChild), New(Size2U)
}
/// Driver object for ID2D1Bitmap1
pub struct Bitmap1(*mut ID2D1Bitmap1); HandleWrapper!(for Bitmap1[ID2D1Bitmap1] + FromRawHandle);
impl DeviceContext
{
    /// Create Bitmap for RenderTarget
    pub fn new_bitmap_for_render_target(&self, src: RenderableBitmapSource, format: dxgi::Format, alpha_mode: dxgi::AlphaMode) -> IOResult<Bitmap1>
    {
        let mut handle = std::ptr::null_mut();
        let props = D2D1_BITMAP_PROPERTIES1
        {
            pixelFormat: D2D1_PIXEL_FORMAT { format, alphaMode: alpha_mode as _ },
            dpiX: 96.0, dpiY: 96.0, colorContext: std::ptr::null_mut(),
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | if let RenderableBitmapSource::FromDxgiSurface(_) = src { D2D1_BITMAP_OPTIONS_CANNOT_DRAW } else { 0 }
        };
        match src
        {
            RenderableBitmapSource::FromDxgiSurface(xs) => unsafe
            {
                (*self.0).CreateBitmapFromDxgiSurface(xs.base()?.as_raw_handle(), &props, &mut handle)
            },
            RenderableBitmapSource::New(size) => unsafe
            {
                (*self.0).CreateBitmap(*size.borrow(), std::ptr::null(), 0, &props, &mut handle)
            }
        }.to_result_with(|| Bitmap1(handle))
    }
    /// Set Render Target
    pub fn set_target<RT: Image + ?Sized>(&self, rt: &RT) -> &Self
    {
        unsafe { (*self.0).SetTarget(rt.as_raw_image()) }; self
    }
    /// Obtain current Render Target
    pub fn get_target(&self) -> ImageRef
    {
        let mut h = std::ptr::null_mut();
        unsafe { (*self.0).GetTarget(&mut h) }; ImageRef(h)
    }
}
pub struct ImageRef(*mut ID2D1Image);
/// Image(2D Pixel Producer) Abstraction
pub trait Image { fn as_raw_image(&self) -> *mut ID2D1Image; }
impl Image for ImageRef { fn as_raw_image(&self) -> *mut ID2D1Image { self.0 } }
impl Image for Bitmap { fn as_raw_image(&self) -> *mut ID2D1Image { self.0 as _ } }
impl Image for Bitmap1 { fn as_raw_image(&self) -> *mut ID2D1Image { self.0 as _ } }

/// Driver object for ID2D1Brush
pub trait Brush { fn as_raw_brush(&self) -> *mut ID2D1Brush; }
/// Driver object for ID2D1SolidColorBrush
pub struct SolidColorBrush(*mut ID2D1SolidColorBrush); HandleWrapper!(for SolidColorBrush[ID2D1SolidColorBrush] + FromRawHandle);
/// Driver object for ID2D1LinearGradientBrush
pub struct LinearGradientBrush(*mut ID2D1LinearGradientBrush); HandleWrapper!(for LinearGradientBrush[ID2D1LinearGradientBrush] + FromRawHandle);
/// Driver object for ID2D1RadialGradientBrush
pub struct RadialGradientBrush(*mut ID2D1RadialGradientBrush); HandleWrapper!(for RadialGradientBrush[ID2D1RadialGradientBrush] + FromRawHandle);
impl Brush for SolidColorBrush { fn as_raw_brush(&self) -> *mut ID2D1Brush { self.0 as _ } }
impl Brush for LinearGradientBrush { fn as_raw_brush(&self) -> *mut ID2D1Brush { self.0 as _ } }
impl Brush for RadialGradientBrush { fn as_raw_brush(&self) -> *mut ID2D1Brush { self.0 as _ } }
/// Driver object for ID2D1GradientStopCollection
pub struct GradientStopCollection(*mut ID2D1GradientStopCollection); HandleWrapper!(for GradientStopCollection[ID2D1GradientStopCollection] + FromRawHandle);
#[repr(C)] #[derive(Clone)]
pub struct GradientStop(pub f32, pub ColorF);
impl Borrow<D2D1_GRADIENT_STOP> for GradientStop { fn borrow(&self) -> &D2D1_GRADIENT_STOP { unsafe { std::mem::transmute(self) } } }
#[repr(C)] #[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gamma { Linear = D2D1_GAMMA_1_0 as _, SRGB = D2D1_GAMMA_2_2 as _ }
#[repr(C)] #[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendMode { Clamp = D2D1_EXTEND_MODE_CLAMP as _, Wrap = D2D1_EXTEND_MODE_WRAP as _, Mirror = D2D1_EXTEND_MODE_MIRROR as _ }

impl SolidColorBrush
{
    pub fn set_color<C: Borrow<D2D1_COLOR_F> + ?Sized>(&self, col: &C) { unsafe { (*self.0).SetColor(col.borrow()); } }
}

/// Driver class for ID2D1PathGeometry
pub struct PathGeometry(*mut ID2D1PathGeometry); HandleWrapper!(for PathGeometry[ID2D1PathGeometry] + FromRawHandle);
impl Factory
{
    pub fn new_path_geometry(&self) -> IOResult<PathGeometry>
    {
        let mut h = std::ptr::null_mut();
        unsafe { (*self.0).CreatePathGeometry(&mut h).to_result_with(|| PathGeometry(h)) }
    }
}
/// Driver class for ID2D1GeometrySink
pub struct GeometrySink(*mut ID2D1GeometrySink); HandleWrapper!(for GeometrySink[ID2D1GeometrySink] + FromRawHandle);
impl PathGeometry
{
    pub fn open(&self) -> IOResult<GeometrySink>
    {
        let mut h = std::ptr::null_mut();
        unsafe { (*self.0).Open(&mut h).to_result_with(|| GeometrySink(h)) }
    }
}

/// Geometry Segment
pub trait GeometrySegment
{
    fn add_to(&self, sink: &GeometrySink);
    fn add_multi(v: &[Self], sink: &GeometrySink) where Self: Sized;
}
impl GeometrySink
{
    pub fn begin_figure<P: Borrow<D2D1_POINT_2F> + ?Sized>(&self, p: &P, fill: bool) -> &Self
    {
        let fb = if fill { D2D1_FIGURE_BEGIN_FILLED } else { D2D1_FIGURE_BEGIN_HOLLOW };
        unsafe { (*self.0).BeginFigure(*p.borrow(), fb) }; self
    }
    pub fn add<S: GeometrySegment + ?Sized>(&self, segment: &S) -> &Self
    {
        segment.add_to(self); self
    }
    pub fn end_figure(&self, close: bool) -> &Self
    {
        let fe = if close { D2D1_FIGURE_END_CLOSED } else { D2D1_FIGURE_END_OPEN };
        unsafe { (*self.0).EndFigure(fe) }; self
    }
    pub fn close(&self) -> IOResult<()> { unsafe { (*self.0).Close().checked() } }
}
pub use winapi::um::d2d1::{
    D2D1_POINT_2F as Point2F, D2D1_ARC_SEGMENT as ArcSegment,
    D2D1_BEZIER_SEGMENT as BezierSegment, D2D1_QUADRATIC_BEZIER_SEGMENT as QuadraticBezierSegment
};
#[repr(C)] #[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum SweepDirection
{
    CCW = D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE as _, CW = D2D1_SWEEP_DIRECTION_CLOCKWISE as _
}
#[repr(C)] #[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum ArcSize
{
    Small = D2D1_ARC_SIZE_SMALL as _, Large = D2D1_ARC_SIZE_LARGE as _
}
impl GeometrySegment for D2D1_ARC_SEGMENT
{
    fn add_to(&self, sink: &GeometrySink) { unsafe { (*sink.0).AddArc(self); } }
    fn add_multi(_: &[Self], _: &GeometrySink) { unimplemented!(); }
}
impl GeometrySegment for D2D1_BEZIER_SEGMENT
{
    fn add_to(&self, sink: &GeometrySink) { unsafe { (*sink.0).AddBezier(self); } }
    fn add_multi(v: &[Self], sink: &GeometrySink) { unsafe { (*sink.0).AddBeziers(v.as_ptr(), v.len() as _); } }
}
use winapi::um::d2d1::D2D1_POINT_2F;
/// Line
impl GeometrySegment for D2D1_POINT_2F
{
    fn add_to(&self, sink: &GeometrySink) { unsafe { (*sink.0).AddLine(*self); } }
    fn add_multi(v: &[Self], sink: &GeometrySink) { unsafe { (*sink.0).AddLines(v.as_ptr(), v.len() as _); } }
}
/// Line
impl GeometrySegment for metrics::Point2F
{
    fn add_to(&self, sink: &GeometrySink) { unsafe { (*sink.0).AddLine(*self.borrow()); } }
    fn add_multi(v: &[Self], sink: &GeometrySink) { unsafe { (*sink.0).AddLines(v.as_ptr() as _, v.len() as _); } }
}
impl GeometrySegment for D2D1_QUADRATIC_BEZIER_SEGMENT
{
    fn add_to(&self, sink: &GeometrySink) { unsafe { (*sink.0).AddQuadraticBezier(self); } }
    fn add_multi(v: &[Self], sink: &GeometrySink) { unsafe { (*sink.0).AddQuadraticBeziers(v.as_ptr(), v.len() as _); } }
}

/// Driver class for ID2D1GaussianBlurEffect
pub struct GaussianBlurEffect(*mut ID2D1Effect); HandleWrapper!(for GaussianBlurEffect[ID2D1Effect] + FromRawHandle);
impl DeviceContext
{
    /// Create Gaussian Blur Effect
    pub fn new_gaussian_blur_effect(&self) -> IOResult<GaussianBlurEffect>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateEffect(&CLSID_D2D1GaussianBlur, &mut handle).to_result_with(|| GaussianBlurEffect(handle)) }
    }
}
impl GaussianBlurEffect
{
    pub fn set_source<I: EffectInput + ?Sized>(&self, input: &I) { self.set_input(0, input); }
    pub fn set_standard_deviation(&self, dev: f32) -> IOResult<()>
    {
        self.set_value(D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION as _, D2D1_PROPERTY_TYPE_UNKNOWN, &dev)
    }
}
/// Defines Effect Input
pub trait EffectInput { fn set_input_for<E: Effect + ?Sized>(&self, fx: &E, index: u32); }
impl<E: Effect + ?Sized> EffectInput for E
{
    fn set_input_for<FX: Effect + ?Sized>(&self, fx: &FX, index: u32) { unsafe { (*fx.as_raw_effect()).SetInput(index, self.get_output().0, true as _); } }
}
impl EffectInput for Bitmap1
{
    fn set_input_for<FX: Effect + ?Sized>(&self, fx: &FX, index: u32) { unsafe { (*fx.as_raw_effect()).SetInput(index, self.0 as *mut _, true as _); } }
}
/// As Effect
pub trait Effect
{
    fn as_raw_effect(&self) -> *mut ID2D1Effect;

    fn set_input<I: EffectInput + ?Sized>(&self, index: usize, input: &I) { input.set_input_for(self, index as _); }
    fn get_output(&self) -> ImageRef
    {
        let mut o = std::ptr::null_mut();
        unsafe { (*self.as_raw_effect()).GetOutput(&mut o) }; ImageRef(o)
    }
    fn set_value<T>(&self, index: usize, ptype: D2D1_PROPERTY_TYPE, value: &T) -> IOResult<()>
    {
        unsafe { (*self.as_raw_effect()).SetValue(index as _, ptype, std::mem::transmute(value), std::mem::size_of::<T>() as _).checked() }
    }
}
impl Effect for GaussianBlurEffect { fn as_raw_effect(&self) -> *mut ID2D1Effect { self.0 } }

/// Matrix 3x2
pub struct Matrix3x2F(D2D1_MATRIX_3X2_F);
impl Matrix3x2F
{
    pub fn unwrap(self) -> D2D1_MATRIX_3X2_F { self.0 }

    pub fn identity() -> Self
    {
        Matrix3x2F(D2D1_MATRIX_3X2_F { matrix: [[1.0, 0.0], [0.0, 1.0], [0.0, 0.0]] })
    }
    pub fn translation(x: f32, y: f32) -> Self
    {
        Matrix3x2F(D2D1_MATRIX_3X2_F { matrix: [[1.0, 0.0], [0.0, 1.0], [x, y]] })
    }
}
use winapi::um::d2d1::D2D1_MATRIX_3X2_F;
impl Borrow<D2D1_MATRIX_3X2_F> for Matrix3x2F { fn borrow(&self) -> &D2D1_MATRIX_3X2_F { &self.0 } }
