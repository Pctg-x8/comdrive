//! DirectComposition Driver

//! ABI Correction Tweaks:
//! - Order of some functions(receiving float or IDCompositionAnimation) in vtable
//!   may be appeared in reversed order against declaration.

use winapi::ctypes::c_float;
use winapi::um::dcomp::*;
use winapi::um::dcompanimation::*;
use winapi::shared::dcomptypes::*;
use winapi::shared::minwindef::BOOL;
use winapi::shared::windef::POINT;
use winapi::shared::dxgi::IDXGISurface;
use winapi::um::d2d1_1::ID2D1DeviceContext;
use super::*;
use metrics::*;

#[repr(C)] #[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum BitmapInterpolationMode
{
    NearestNeighbor = DCOMPOSITION_BITMAP_INTERPOLATION_MODE_NEAREST_NEIGHBOR as _,
    Linear = DCOMPOSITION_BITMAP_INTERPOLATION_MODE_LINEAR as _
}

/// Driver object for IDCompositionDesktopDevice
pub struct Device(*mut IDCompositionDesktopDevice); HandleWrapper!(for Device[IDCompositionDesktopDevice]);
impl Device
{
    /// Create
    pub fn new(render_device: Option<&AsIUnknown>) -> IOResult<Self>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { DCompositionCreateDevice3(render_device.map(AsIUnknown::as_iunknown).unwrap_or(std::ptr::null_mut()),
            &IDCompositionDesktopDevice::uuidof(), &mut handle).to_result_with(|| Device(handle as _)) }
    }
    /// Commit Changes
    pub fn commit(&self) -> IOResult<()> { unsafe { (*self.0).Commit().checked() } }
}

/// Driver object for IDCompositionTarget
pub struct Target(*mut IDCompositionTarget); HandleWrapper!(for Target[IDCompositionTarget]);
pub trait TargetProvider<TargetBaseObject, TargetType: Handle<RawType = IDCompositionTarget>>
{
    fn new_target_for(&self, target_base: &TargetBaseObject) -> IOResult<TargetType>;
}
impl TargetProvider<HWND, Target> for Device
{
    fn new_target_for(&self, target_base: &HWND) -> IOResult<Target>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateTargetForHwnd(*target_base, true as BOOL, &mut handle).to_result_with(|| Target(handle)) }
    }
}
impl Target
{
    /// Set Root Visual
    pub fn set_root(&self, new_root: &Visual) -> IOResult<()>
    {
        unsafe { (*self.0).SetRoot(new_root.0 as _).checked() }
    }
}

/// Transparent Selector of f32(immediate value) or IDCompositionAnimation
pub trait Parameter: Sized
{
    fn pass<F, FA>(self, setter_f: F, setter_a: FA) -> IOResult<()>
        where F: FnOnce(f32) -> HRESULT, FA: FnOnce(*mut IDCompositionAnimation) -> HRESULT;
}
impl Parameter for f32
{
    fn pass<F, FA>(self, setter_f: F, _: FA) -> IOResult<()>
        where F: FnOnce(f32) -> HRESULT, FA: FnOnce(*mut IDCompositionAnimation) -> HRESULT { setter_f(self).checked() }
}
macro_rules! ObtainPropertySetter
{
    (extern fn ($this: ident: $caller: ty, $($pt: ty => $name: ident)|*) -> $rval: ty) =>
    {
        ($(std::mem::transmute::<_, unsafe extern "system" fn($caller, $pt) -> $rval>((*(*$this.0).lpVtbl).$name)),*)
    };
    (extern fn (v [$vtbl: expr] $caller: ty, $($pt: ty => $name: ident)|*) -> $rval: ty) =>
    {
        ($(std::mem::transmute::<_, unsafe extern "system" fn($caller, $pt) -> $rval>($vtbl.$name)),*)
    }
}

/// Driver object for IDCompositionVisual3
pub struct Visual(*mut IDCompositionVisual3); HandleWrapper!(for Visual[IDCompositionVisual3] + FromRawHandle);
struct Visual2(*mut IDCompositionVisual2); HandleWrapper!(for Visual2[IDCompositionVisual2]);
impl Device
{
    /// Create Visual
    pub fn new_visual(&self) -> IOResult<Visual>
    {
        let mut handle = std::ptr::null_mut();
        let h2 = unsafe { (*self.0).CreateVisual(&mut handle).to_result_with(|| Visual2(handle))? };
        h2.query_interface()
    }
}
impl Visual
{
    /// Insert child visual
    pub fn insert_child(&self, child: &Visual, at: InsertAt) -> IOResult<()>
    {
        match at
        {
            InsertAt::Top => unsafe { (*self.0).AddVisual(child.0 as _, false as BOOL, std::ptr::null_mut()) },
            InsertAt::Bottom => unsafe { (*self.0).AddVisual(child.0 as _, true as BOOL, std::ptr::null_mut()) },
            InsertAt::Above(rv) => unsafe { (*self.0).AddVisual(child.0 as _, true as BOOL, rv.0 as _) },
            InsertAt::Below(rv) => unsafe { (*self.0).AddVisual(child.0 as _, false as BOOL, rv.0 as _) }
        }.checked()
    }
    /// Remove specified child
    pub fn remove_child(&self, child: &Visual) -> IOResult<()> { unsafe { (*self.0).RemoveVisual(child.0 as _).checked() } }
    /// Remove all of children
    pub fn remove_all_children(&self) -> IOResult<()> { unsafe { (*self.0).RemoveAllVisuals().checked() } }

    /// Set Transform
    pub fn set_transform<T: Transform>(&self, transform: &T) -> IOResult<()>
    {
        let vtbl = unsafe { &(*(*self.0).lpVtbl).parent.parent.parent };
        let to = unsafe { ObtainPropertySetter!(extern fn(v [vtbl] *mut IDCompositionVisual, *const IDCompositionTransform => SetTransform_1) -> HRESULT) };
        unsafe { to(self.0 as _, transform.as_raw_transform()).checked() }
    }
    /// Set Effect
    pub fn set_effect<E: Effect>(&self, effect: &E) -> IOResult<()> { unsafe { (*self.0).SetEffect(effect.as_raw_effect()).checked() } }
    /// Set Content
    pub fn set_content(&self, content: Option<&AsIUnknown>) -> IOResult<()>
    {
        unsafe { (*self.0).SetContent(content.map(AsIUnknown::as_iunknown).unwrap_or(std::ptr::null_mut())).checked() }
    }
    /// Set X Offset
    pub fn set_left<P: Parameter>(&self, v: P) -> IOResult<()>
    {
        let vtbl = unsafe { &(*(*self.0).lpVtbl).parent.parent.parent };
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(v [vtbl] *mut IDCompositionVisual, *const IDCompositionAnimation => SetOffsetX_2 | c_float => SetOffsetX_1) -> HRESULT) };
        v.pass(|x| unsafe { fpv(self.0 as _, x) }, |x| unsafe { fpo(self.0 as _, x) })
    }
    /// Set Y Offset
    pub fn set_top<P: Parameter>(&self, v: P) -> IOResult<()>
    {
        let vtbl = unsafe { &(*(*self.0).lpVtbl).parent.parent.parent };
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(v [vtbl] *mut IDCompositionVisual, *const IDCompositionAnimation => SetOffsetY_2 | c_float => SetOffsetY_1) -> HRESULT) };
        v.pass(|x| unsafe { fpv(self.0 as _, x) }, |x| unsafe { fpo(self.0 as _, x) })
    }
    /// Set Offset
    pub fn set_offset<Px: Parameter, Py: Parameter>(&self, x: Px, y: Py) -> IOResult<()>
    {
        self.set_left(x).and_then(|_| self.set_top(y))
    }
    /// Set Opacity
    pub fn set_opacity<P: Parameter>(&self, a: P) -> IOResult<()>
    {
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(self: *mut IDCompositionVisual3, *const IDCompositionAnimation => SetOpacity_2 | c_float => SetOpacity_1) -> HRESULT) };
        a.pass(|x| unsafe { fpv(self.0, x) }, |x| unsafe { fpo(self.0, x) })
    }
    /// 拡縮時のビットマップ補間モードを指定する
    pub fn set_bitmap_interpolation(&self, ip: BitmapInterpolationMode) -> IOResult<()>
    {
        unsafe { (*self.0).SetBitmapInterpolationMode(ip as _).checked() }
    }
}
/// Insertion Mode
pub enum InsertAt<'a> { Top, Bottom, Above(&'a Visual), Below(&'a Visual) }

/// Transform
pub trait Transform { fn as_raw_transform(&self) -> *const IDCompositionTransform; }
/// Effect
pub trait Effect { fn as_raw_effect(&self) -> *const IDCompositionEffect; }
impl<T: Transform> Effect for T { fn as_raw_effect(&self) -> *const IDCompositionEffect { self.as_raw_transform() as _ } }

/// Driver object for IDCompositionScaleTransform
pub struct ScaleTransform(*mut IDCompositionScaleTransform); HandleWrapper!(for ScaleTransform[IDCompositionScaleTransform]);
/// Driver object for IDCompositionRotateTransform
pub struct RotateTransform(*mut IDCompositionRotateTransform); HandleWrapper!(for RotateTransform[IDCompositionRotateTransform]);
impl Device
{
    /// Create Scale Transform
    pub fn new_scale_transform(&self) -> IOResult<ScaleTransform>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateScaleTransform(&mut handle) }.to_result_with(|| ScaleTransform(handle))
    }
    /// Create Rotate Transform
    pub fn new_rotate_transform(&self) -> IOResult<RotateTransform>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateRotateTransform(&mut handle) }.to_result_with(|| RotateTransform(handle))
    }
}
impl ScaleTransform
{
    /// Set X Scaling
    pub fn set_x_scale<P: Parameter>(&self, v: P) -> IOResult<()>
    {
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(self: *mut IDCompositionScaleTransform, *const IDCompositionAnimation => SetScaleX_2 | c_float => SetScaleX_1) -> HRESULT) };
        v.pass(|x| unsafe { fpv(self.0 as _, x) }, |x| unsafe { fpo(self.0 as _, x) })
    }
    /// Set Y Scaling
    pub fn set_y_scale<P: Parameter>(&self, v: P) -> IOResult<()>
    {
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(self: *mut IDCompositionScaleTransform, *const IDCompositionAnimation => SetScaleY_2 | c_float => SetScaleY_1) -> HRESULT) };
        v.pass(|x| unsafe { fpv(self.0 as _, x) }, |x| unsafe { fpo(self.0 as _, x) })
    }
    /// Set Both parameter
    pub fn set<Px: Parameter, Py: Parameter>(&self, x: Px, y: Py) -> IOResult<()>
    {
        self.set_x_scale(x).and_then(|_| self.set_y_scale(y))
    }
}
impl RotateTransform
{
    /// Set Angle
    pub fn set_angle<P: Parameter>(&self, v: P) -> IOResult<()>
    {
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(self: *mut IDCompositionRotateTransform, *const IDCompositionAnimation => SetAngle_2 | c_float => SetAngle_1) -> HRESULT) };
        v.pass(|x| unsafe { fpv(self.0 as _, x) }, |x| unsafe { fpo(self.0 as _, x) })
    }
    /// Set Center X
    pub fn set_center_x<P: Parameter>(&self, v: P) -> IOResult<()>
    {
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(self: *mut IDCompositionRotateTransform, *const IDCompositionAnimation => SetCenterX_2 | c_float => SetCenterX_1) -> HRESULT) };
        v.pass(|x| unsafe { fpv(self.0 as _, x) }, |x| unsafe { fpo(self.0 as _, x) })
    }
    /// Set Center Y
    pub fn set_center_y<P: Parameter>(&self, v: P) -> IOResult<()>
    {
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(self: *mut IDCompositionRotateTransform, *const IDCompositionAnimation => SetCenterY_2 | c_float => SetCenterY_1) -> HRESULT) };
        v.pass(|x| unsafe { fpv(self.0 as _, x) }, |x| unsafe { fpo(self.0 as _, x) })
    }
    /// Set Center Parameter
    pub fn set_center<Px: Parameter, Py: Parameter>(&self, x: Px, y: Py) -> IOResult<()>
    {
        self.set_center_x(x).and_then(|_| self.set_center_y(y))
    }
}
impl Transform for ScaleTransform  { fn as_raw_transform(&self) -> *const IDCompositionTransform { self.0 as _ } }
impl Transform for RotateTransform { fn as_raw_transform(&self) -> *const IDCompositionTransform { self.0 as _ } }
/// Driver object for IDCompositionTransform(Group)
pub struct TransformGroup(*mut IDCompositionTransform); HandleWrapper!(for TransformGroup[IDCompositionTransform]);
impl Device
{
    /// Make Group of Transforms
    pub fn group_transforms(&self, tfs: &[&Transform]) -> IOResult<TransformGroup>
    {
        let tfs = tfs.into_iter().map(|t| t.as_raw_transform()).collect::<Vec<_>>();
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateTransformGroup(tfs.as_ptr() as *mut _, tfs.len() as _, &mut handle) }.to_result_with(|| TransformGroup(handle))
    }
}
impl Transform for TransformGroup { fn as_raw_transform(&self) -> *const IDCompositionTransform { self.0 } }

/// Driver object for IDCompositionSurfaceFactory for Direct2D
pub struct SurfaceFactory2(*mut IDCompositionSurfaceFactory); HandleWrapper!(for SurfaceFactory2[IDCompositionSurfaceFactory] + FromRawHandle);
/// Driver object for IDCompositionSurfaceFactory for Direct3D
pub struct SurfaceFactory3(*mut IDCompositionSurfaceFactory); HandleWrapper!(for SurfaceFactory3[IDCompositionSurfaceFactory] + FromRawHandle);
pub trait SurfaceFactoryProvider<RenderDevice: AsIUnknown, FactoryType> : AsRawHandle<IDCompositionDesktopDevice>
    where FactoryType: AsRawHandle<IDCompositionSurfaceFactory> + FromRawHandle<IDCompositionSurfaceFactory>
{
    fn new_surface_factory(&self, render_device: &RenderDevice) -> IOResult<FactoryType>
    {
        let mut handle = std::ptr::null_mut();
        unsafe
        {
            (*self.as_raw_handle()).CreateSurfaceFactory(render_device.as_iunknown(), &mut handle).to_result_with(|| FactoryType::from_raw_handle(handle))
        }
    }
}
impl SurfaceFactoryProvider<d2::Device, SurfaceFactory2> for Device {}
impl SurfaceFactoryProvider<d3d11::Device, SurfaceFactory3> for Device {}
/// Driver object for IDCompositionSurface for Direct2D
pub struct Surface2(*mut IDCompositionSurface); HandleWrapper!(for Surface2[IDCompositionSurface]);
/// Driver object for IDCompositionSurface for Direct3D
pub struct Surface3(*mut IDCompositionSurface); HandleWrapper!(for Surface3[IDCompositionSurface]);
pub trait SurfaceFactory
{
    type Surface: Handle<RawType = IDCompositionSurface>;
    fn new_surface(&self, init_size: &Size2U, pixel_format: dxgi::Format, alpha_mode: dxgi::AlphaMode) -> IOResult<Self::Surface>;
}
impl SurfaceFactory for SurfaceFactory2
{
    type Surface = Surface2;
    /// Create Surface
    fn new_surface(&self, init_size: &Size2U, pixel_format: dxgi::Format, alpha_mode: dxgi::AlphaMode) -> IOResult<Surface2>
    {
        let mut handle = std::ptr::null_mut();
        unsafe
        { 
            (*self.0).CreateSurface(init_size.width(), init_size.height(), pixel_format as _, alpha_mode as _, &mut handle).to_result_with(|| Surface2(handle))
        }
    }
}
impl SurfaceFactory for SurfaceFactory3
{
    type Surface = Surface3;
    /// Create Surface
    fn new_surface(&self, init_size: &Size2U, pixel_format: dxgi::Format, alpha_mode: dxgi::AlphaMode) -> IOResult<Surface3>
    {
        let mut handle = std::ptr::null_mut();
        unsafe
        {
            (*self.0).CreateSurface(init_size.width(), init_size.height(), pixel_format as _, alpha_mode as _, &mut handle).to_result_with(|| Surface3(handle))
        }
    }
}
pub trait Surface<'s>
{
    type RenderContext: 's;
    fn begin_draw(&'s self) -> IOResult<Self::RenderContext>;
}

pub struct SurfaceRenderContext2<'s>(&'s Surface2, d2::DeviceContext, POINT);
pub struct SurfaceRenderContext3<'s>(&'s Surface3, d3d11::Texture2D, POINT);
impl<'s> Surface<'s> for Surface2
{
    type RenderContext = SurfaceRenderContext2<'s>;
    fn begin_draw(&'s self) -> IOResult<SurfaceRenderContext2<'s>>
    {
        let (mut handle, mut offs) = (std::ptr::null_mut(), unsafe { std::mem::uninitialized() });
        unsafe { (*self.0).BeginDraw(std::ptr::null(), &ID2D1DeviceContext::uuidof(), &mut handle, &mut offs) }
            .to_result_with(|| SurfaceRenderContext2(&self, unsafe { d2::DeviceContext::from_raw_handle(handle as _) }, offs))
    }
}
impl<'s> Surface<'s> for Surface3
{
    type RenderContext = SurfaceRenderContext3<'s>;
    fn begin_draw(&'s self) -> IOResult<SurfaceRenderContext3<'s>>
    {
        let (mut xt, mut offs) = (std::ptr::null_mut(), unsafe { std::mem::uninitialized() });
        let xt = unsafe { (*self.0).BeginDraw(std::ptr::null(), &IDXGISurface::uuidof(), &mut xt, &mut offs) }
            .to_result_with(|| unsafe { dxgi::Surface::from_raw_handle(xt as _) })?;
        xt.query_interface().map(|t| SurfaceRenderContext3(&self, t, offs))
    }
}
impl<'s> SurfaceRenderContext2<'s>
{
    pub fn apply_offset(self) -> Self
    {
        self.1.set_transform(&d2::Matrix3x2F::translation(self.2.x as f32, self.2.y as f32));
        self
    }
    pub fn renderer(&self) -> &d2::DeviceContext { &self.1 }
    pub fn offset(&self) -> Point2 { self.2.into() }
}
impl<'s> SurfaceRenderContext3<'s>
{
    pub fn render_target(&self) -> &d3d11::Texture2D { &self.1 }
    pub fn offset(&self) -> Point2 { self.2.into() }
}
impl<'s> Drop for SurfaceRenderContext2<'s> { fn drop(&mut self) { unsafe { (*(self.0).0).EndDraw().checked().unwrap() }; } }
impl<'s> Drop for SurfaceRenderContext3<'s> { fn drop(&mut self) { unsafe { (*(self.0).0).EndDraw().checked().unwrap() }; } }

pub struct EffectFactory(*mut IDCompositionDevice3); HandleWrapper!(for EffectFactory[IDCompositionDevice3] + FromRawHandle);
impl Device
{
    pub fn effect_factory(&self) -> IOResult<EffectFactory> { self.query_interface() }
}

/// Gaussian Blur Effect
pub struct GaussianBlurEffect(*mut IDCompositionGaussianBlurEffect); HandleWrapper!(for GaussianBlurEffect[IDCompositionGaussianBlurEffect]);
impl Effect for GaussianBlurEffect { fn as_raw_effect(&self) -> *const IDCompositionEffect { self.0 as _ } }
impl EffectFactory
{
    pub fn new_gaussian_blur_effect(&self) -> IOResult<GaussianBlurEffect>
    {
        let mut h = std::ptr::null_mut();
        unsafe { (*self.0).CreateGaussianBlurEffect(&mut h) }.to_result_with(|| GaussianBlurEffect(h))
    }
}
impl GaussianBlurEffect
{
    pub fn set_standard_deviation<P: Parameter>(&self, param: P) -> IOResult<()>
    {
        let (fpo, fpv) = unsafe { ObtainPropertySetter!(extern fn(self: *mut IDCompositionGaussianBlurEffect,
            *const IDCompositionAnimation => SetStandardDeviation_1 | c_float => SetStandardDeviation_2) -> HRESULT) };
        param.pass(|x| unsafe { fpv(self.0, x) }, |x| unsafe { fpo(self.0, x) })
    }
}
