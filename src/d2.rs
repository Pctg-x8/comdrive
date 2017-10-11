//! Direct2D Driver

use super::*;
use winapi::um::d2d1::*;
use winapi::um::d2d1_1::*;
use winapi::um::d2d1effects::*;
use winapi::um::dcommon::*;
use winapi::shared::dxgiformat::DXGI_FORMAT_UNKNOWN;
use std::ptr::{null, null_mut};
use metrics::*;
use std::convert::AsRef;

pub use winapi::um::d2d1::{D2D1_COLOR_F as ColorF, D2D1_SIZE_F as SizeF};
#[repr(C)] #[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntialiasMode
{
    Aliased = D2D1_ANTIALIAS_MODE_ALIASED as _, PerPrimitive = D2D1_ANTIALIAS_MODE_PER_PRIMITIVE as _
}

/// Driver object for ID2D1Factory
pub struct Factory(*mut ID2D1Factory);
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
pub struct Device(*mut ID2D1Device);
impl AsIUnknown for Device { fn as_iunknown(&self) -> *mut IUnknown { self.0 as _ } }
impl Device
{
    /// Create on Direct3D Device
    pub fn new<DC: dxgi::DeviceChild>(dev3: &DC) -> IOResult<Self>
    {
        let cp = D2D1_CREATION_PROPERTIES
        {
            debugLevel: D2D1_DEBUG_LEVEL_WARNING,
            threadingMode: D2D1_THREADING_MODE_SINGLE_THREADED,
            options: D2D1_DEVICE_CONTEXT_OPTIONS_NONE
        };
        let mut handle = std::ptr::null_mut();
        dev3.parent().and_then(|dx| unsafe
        {
            D2D1CreateDevice(dx.as_raw_handle() as _, &cp, &mut handle)
        }.to_result_with(|| Device(handle)))
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
pub struct HwndRenderTarget(*mut ID2D1HwndRenderTarget);
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
        unsafe { (*self.0).CreateHwndRenderTarget(&rtprops, &hwrtprops, &mut handle) }.to_result_with(|| HwndRenderTarget(handle))
    }
}

/// Driver object for ID2D1DeviceContext
pub struct DeviceContext(*mut ID2D1DeviceContext);
impl Handle for DeviceContext
{
    type RawType = ID2D1DeviceContext;
    fn query_interface<Q: Handle>(&self) -> IOResult<Q> where Q: FromRawHandle<<Q as Handle>::RawType>
    {
        let mut handle: *mut Q::RawType = std::ptr::null_mut();
        unsafe { (*self.0).QueryInterface(&Q::RawType::uuidof(), std::mem::transmute(&mut handle)) }.to_result_with(|| unsafe { Q::from_raw_handle(handle) })
    }
}
impl AsRawHandle<ID2D1DeviceContext> for DeviceContext { fn as_raw_handle(&self) -> *mut ID2D1DeviceContext { self.0 } }
impl FromRawHandle<ID2D1DeviceContext> for DeviceContext { unsafe fn from_raw_handle(h: *mut ID2D1DeviceContext) -> Self { DeviceContext(h) } }
impl AsIUnknown for DeviceContext { fn as_iunknown(&self) -> *mut IUnknown { self.0 as _ } }
impl Device
{
    pub fn new_context(&self) -> IOResult<DeviceContext>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_ENABLE_MULTITHREADED_OPTIMIZATIONS, &mut handle) }
            .to_result_with(|| DeviceContext(handle))
    }
}

/// RenderTarget系の共通実装
pub trait RenderTarget
{
    /// コンテキストハンドル
    fn as_rt_handle(&self) -> *mut ID2D1RenderTarget;

    /// 描画開始
    fn begin_draw(&self) -> &Self { unsafe { (*self.as_rt_handle()).BeginDraw() }; self }
    /// 描画終了
    fn end_draw(&self) -> IOResult<()> { unsafe { (*self.as_rt_handle()).EndDraw(null_mut(), null_mut()) }.checked() }
    /// クリップ範囲の設定
    fn push_aa_clip(&self, rect: &Rect2F, aliasing: AntialiasMode) -> &Self
    {
        unsafe { (*self.as_rt_handle()).PushAxisAlignedClip(transmute_safe(rect), aliasing as _) }; self
    }
    /// クリップ範囲を解除
    fn pop_aa_clip(&self) -> &Self { unsafe { (*self.as_rt_handle()).PopAxisAlignedClip() }; self }
    
    /// トランスフォーム行列をセット
    fn set_transform<Matrix: AsRef<D2D1_MATRIX_3X2_F>>(&self, matrix: &Matrix) -> &Self
    {
        unsafe { (*self.as_rt_handle()).SetTransform(matrix.as_ref()) }; self
    }
    /// 描画ターゲットの中身を消去
    fn clear(&self, color: &ColorF) -> &Self { unsafe { (*self.as_rt_handle()).Clear(color) }; self }

    /// 矩形を塗りつぶし
    fn fill_rect<B: Brush + ?Sized>(&self, area: &Rect2F, brush: &B) -> &Self
    {
        unsafe { (*self.as_rt_handle()).FillRectangle(transmute_safe(area), brush.as_raw_brush()) }; self
    }
    /// 線を引く
    fn draw_line<B: Brush + ?Sized>(&self, start: metrics::Point2F, end: metrics::Point2F, brush: &B) -> &Self
    {
        unsafe { (*self.as_rt_handle()).DrawLine(*transmute_safe(&start), *transmute_safe(&end), brush.as_raw_brush(), 1.0, null_mut()) };
        self
    }
    /// レイアウト済みテキストの描画
    fn draw_text<B: Brush + ?Sized>(&self, p: metrics::Point2F, layout: &dwrite::TextLayout, brush: &B) -> &Self
    {
        unsafe { (*self.as_rt_handle()).DrawTextLayout(*transmute_safe(&p), layout.as_raw_handle() as _, brush.as_raw_brush(), D2D1_DRAW_TEXT_OPTIONS_NONE) };
        self
    }

    /// ビットマップを描く
    fn draw_bitmap(&self, bmp: &Bitmap, rect: &Rect2F) -> &Self
    {
        unsafe { (*self.as_rt_handle()).DrawBitmap(bmp.0, transmute_safe(rect), 1.0, D2D1_INTERPOLATION_MODE_LINEAR, null()) };
        self
    }
}

impl RenderTarget for HwndRenderTarget { fn as_rt_handle(&self) -> *mut ID2D1RenderTarget { self.0 as _ } }
impl RenderTarget for DeviceContext { fn as_rt_handle(&self) -> *mut ID2D1RenderTarget { self.0 as _ } }
impl DeviceContext
{
    /// Imageを描く
    pub fn draw<IMG: Image>(&self, offs: metrics::Point2F, image: &IMG) -> &Self
    {
        unsafe { (*self.0).DrawImage(image.as_raw_image(), transmute_safe(&offs), std::ptr::null(), D2D1_INTERPOLATION_MODE_LINEAR, D2D1_COMPOSITE_MODE_SOURCE_OVER) };
        self
    }
    /// Effectを描く
    pub fn draw_effected<E: Effect>(&self, offs: metrics::Point2F, fx: &E) -> &Self
    {
        self.draw(offs, &fx.get_output())
    }
}
/// Driver object for ID2D1Bitmap(Context bound object)
pub struct Bitmap(*mut ID2D1Bitmap);
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
pub struct Bitmap1(*mut ID2D1Bitmap1);
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
            bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | if let &RenderableBitmapSource::FromDxgiSurface(_) = &src { D2D1_BITMAP_OPTIONS_CANNOT_DRAW } else { 0 }
        };
        match src
        {
            RenderableBitmapSource::FromDxgiSurface(xs) => unsafe
            {
                (*self.0).CreateBitmapFromDxgiSurface(xs.base()?.as_raw_handle(), &props, &mut handle)
            },
            RenderableBitmapSource::New(size) => unsafe
            {
                (*self.0).CreateBitmap(*transmute_safe(&size), std::ptr::null(), 0, &props, &mut handle)
            }
        }.to_result_with(|| Bitmap1(handle))
    }
    /// Set Render Target
    pub fn set_target<RT: Image>(&self, rt: &RT) -> &Self
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
pub struct SolidColorBrush(*mut ID2D1SolidColorBrush);
/// Driver object for ID2D1LinearGradientBrush
pub struct LinearGradientBrush(*mut ID2D1LinearGradientBrush);
/// Driver object for ID2D1RadialGradientBrush
pub struct RadialGradientBrush(*mut ID2D1RadialGradientBrush);
impl DeviceContext
{
    /// Create Solid Color Brush
    pub fn new_solid_color_brush(&self, color: &ColorF) -> IOResult<SolidColorBrush>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateSolidColorBrush(color, std::ptr::null(), &mut handle) }.to_result_with(|| SolidColorBrush(handle))
    }
    /// Create Linear Gradient Brush
    pub fn new_linear_gradient_brush(&self, from: metrics::Point2F, to: metrics::Point2F, stops: &GradientStopCollection) -> IOResult<LinearGradientBrush>
    {
        let mut handle = std::ptr::null_mut();
        let lb_props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES
        {
            startPoint: *transmute_safe(&from), endPoint: *transmute_safe(&to)
        };
        let brush_props = D2D1_BRUSH_PROPERTIES { opacity: 1.0, transform: Matrix3x2F::identity().unwrap() };
        unsafe { (*self.0).CreateLinearGradientBrush(&lb_props, &brush_props, stops.0, &mut handle) }
            .to_result_with(|| LinearGradientBrush(handle))
    }
    /// Create Radial Gradient Brush
    pub fn new_radial_gradient_brush(&self, center: Point2F, radius: Size2F, stops: &GradientStopCollection) -> IOResult<RadialGradientBrush>
    {
        let mut handle = std::ptr::null_mut();
        let rb_props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES
        {
            center: unsafe { std::mem::transmute_copy(&center) }, radiusX: radius.x(), radiusY: radius.y(),
            gradientOriginOffset: D2D1_POINT_2F { x: 0.0, y: 0.0 }
        };
        let brush_props = D2D1_BRUSH_PROPERTIES { opacity: 1.0, transform: Matrix3x2F::identity().unwrap() };
        unsafe { (*self.0).CreateRadialGradientBrush(&rb_props, &brush_props, stops.0, &mut handle) }.to_result_with(|| RadialGradientBrush(handle))
    }
}
impl Brush for SolidColorBrush { fn as_raw_brush(&self) -> *mut ID2D1Brush { self.0 as _ } }
impl Brush for LinearGradientBrush { fn as_raw_brush(&self) -> *mut ID2D1Brush { self.0 as _ } }
impl Brush for RadialGradientBrush { fn as_raw_brush(&self) -> *mut ID2D1Brush { self.0 as _ } }
/// Driver object for ID2D1GradientStopCollection
pub struct GradientStopCollection(*mut ID2D1GradientStopCollection);
impl DeviceContext
{
    /// Create Gradient Stop Collection
    pub fn new_gradient_stop_collection(&self, stops: &[GradientStop], gamma: Gamma, extend_mode: ExtendMode) -> IOResult<GradientStopCollection>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { ((*(*self.0).lpVtbl).parent.CreateGradientStopCollection)(self.0 as _, stops.as_ptr() as *const _, stops.len() as _,
            gamma as _, extend_mode as _, &mut handle) }.to_result_with(|| GradientStopCollection(handle))
    }
}
pub struct GradientStop(pub f32, pub ColorF);
unsafe impl MarkForSameBits<D2D1_GRADIENT_STOP> for GradientStop {}
#[repr(C)] #[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gamma { Linear = D2D1_GAMMA_1_0 as _, SRGB = D2D1_GAMMA_2_2 as _ }
#[repr(C)] #[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendMode { Clamp = D2D1_EXTEND_MODE_CLAMP as _, Wrap = D2D1_EXTEND_MODE_WRAP as _, Mirror = D2D1_EXTEND_MODE_MIRROR as _ }

/// Driver class for ID2D1PathGeometry
pub struct PathGeometry(*mut ID2D1PathGeometry);
impl Factory
{
    pub fn new_path_geometry(&self) -> IOResult<PathGeometry>
    {
        let mut h = std::ptr::null_mut();
        unsafe { (*self.0).CreatePathGeometry(&mut h) }.to_result_with(|| PathGeometry(h))
    }
}
/// Driver class for ID2D1GeometrySink
pub struct GeometrySink(*mut ID2D1GeometrySink);
impl PathGeometry
{
    pub fn open(&self) -> IOResult<GeometrySink>
    {
        let mut h = std::ptr::null_mut();
        unsafe { (*self.0).Open(&mut h) }.to_result_with(|| GeometrySink(h))
    }
}

/// Geometry Segment
pub trait GeometrySegment: Sized
{
    fn add_to(&self, sink: &GeometrySink);
    fn add_multi(v: &[Self], sink: &GeometrySink);
}
impl GeometrySink
{
    pub fn begin_figure(&self, p: metrics::Point2F, fill: bool) -> &Self
    {
        let fb = if fill { D2D1_FIGURE_BEGIN_FILLED } else { D2D1_FIGURE_BEGIN_HOLLOW };
        unsafe { (*self.0).BeginFigure(*transmute_safe(&p), fb) }; self
    }
    pub fn add<S: GeometrySegment>(&self, segment: &S) -> &Self
    {
        segment.add_to(self); self
    }
    pub fn end_figure(&self, close: bool) -> &Self
    {
        let fe = if close { D2D1_FIGURE_END_CLOSED } else { D2D1_FIGURE_END_OPEN };
        unsafe { (*self.0).EndFigure(fe) }; self
    }
    pub fn close(&self) -> IOResult<()>
    {
        unsafe { (*self.0).Close() }.checked()
    }
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
    fn add_multi(v: &[Self], sink: &GeometrySink)
    {
        unsafe { (*sink.0).AddBeziers(v.as_ptr(), v.len() as _); }
    }
}
/// Line
impl GeometrySegment for D2D1_POINT_2F
{
    fn add_to(&self, sink: &GeometrySink) { unsafe { (*sink.0).AddLine(*self); } }
    fn add_multi(v: &[Self], sink: &GeometrySink)
    {
        unsafe { (*sink.0).AddLines(v.as_ptr(), v.len() as _); }
    }
}
/// Line
impl GeometrySegment for metrics::Point2F
{
    fn add_to(&self, sink: &GeometrySink) { unsafe { (*sink.0).AddLine(*transmute_safe(self)); } }
    fn add_multi(v: &[Self], sink: &GeometrySink)
    {
        unsafe { (*sink.0).AddLines(v.as_ptr() as _, v.len() as _); }
    }
}
impl GeometrySegment for D2D1_QUADRATIC_BEZIER_SEGMENT
{
    fn add_to(&self, sink: &GeometrySink) { unsafe { (*sink.0).AddQuadraticBezier(self); } }
    fn add_multi(v: &[Self], sink: &GeometrySink)
    {
        unsafe { (*sink.0).AddQuadraticBeziers(v.as_ptr(), v.len() as _); }
    }
}

/// Driver class for ID2D1GaussianBlurEffect
pub struct GaussianBlurEffect(*mut ID2D1Effect);
impl DeviceContext
{
    /// Create Gaussian Blur Effect
    pub fn new_gaussian_blur_effect(&self) -> IOResult<GaussianBlurEffect>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateEffect(&CLSID_D2D1GaussianBlur, &mut handle) }.to_result_with(|| GaussianBlurEffect(handle))
    }
}
impl GaussianBlurEffect
{
    pub fn set_source<I: EffectInput>(&self, input: &I) { self.set_input(0, input); }
    pub fn set_standard_deviation(&self, dev: f32) -> IOResult<()>
    {
        self.set_value(D2D1_GAUSSIANBLUR_PROP_STANDARD_DEVIATION as _, D2D1_PROPERTY_TYPE_UNKNOWN, &dev)
    }
}
/// Defines Effect Input
pub trait EffectInput { unsafe fn set_input_to(&self, fx: *mut ID2D1Effect, index: u32); }
impl<E: Effect> EffectInput for E
{
    unsafe fn set_input_to(&self, fx: *mut ID2D1Effect, index: u32) { (*fx).SetInput(index, self.get_output().0, true as _); }
}
impl EffectInput for Bitmap1
{
    unsafe fn set_input_to(&self, fx: *mut ID2D1Effect, index: u32) { (*fx).SetInput(index, self.0 as *mut _, true as _); }
}
/// As Effect
pub trait Effect
{
    fn as_raw_effect(&self) -> *mut ID2D1Effect;

    fn set_input<I: EffectInput>(&self, index: usize, input: &I) { unsafe { input.set_input_to(self.as_raw_effect(), index as _); } }
    fn get_output(&self) -> ImageRef
    {
        let mut o = std::ptr::null_mut();
        unsafe { (*self.as_raw_effect()).GetOutput(&mut o) }; ImageRef(o)
    }
    fn set_value<T>(&self, index: usize, ptype: D2D1_PROPERTY_TYPE, value: &T) -> IOResult<()>
    {
        unsafe { (*self.as_raw_effect()).SetValue(index as _, ptype, std::mem::transmute(value), std::mem::size_of::<T>() as _) }.checked()
    }
}
impl Effect for GaussianBlurEffect { fn as_raw_effect(&self) -> *mut ID2D1Effect { self.0 } }

AutoRemover!(for Device[ID2D1Device], DeviceContext[ID2D1DeviceContext], Bitmap[ID2D1Bitmap], Bitmap1[ID2D1Bitmap1]);
AutoRemover!(for SolidColorBrush[ID2D1SolidColorBrush], LinearGradientBrush[ID2D1LinearGradientBrush], RadialGradientBrush[ID2D1RadialGradientBrush]);
AutoRemover!(for GradientStopCollection[ID2D1GradientStopCollection], ImageRef[ID2D1Image], GaussianBlurEffect[ID2D1GaussianBlurEffect]);
AutoRemover!(for PathGeometry[ID2D1PathGeometry], GeometrySink[ID2D1GeometrySink]);

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
impl AsRef<D2D1_MATRIX_3X2_F> for Matrix3x2F
{
    fn as_ref(&self) -> &D2D1_MATRIX_3X2_F { &self.0 }
}
