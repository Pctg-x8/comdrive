//! Direct3D 11 on 12

use winapi::um::d3d11on12::*;
use winapi::um::d3d11::*;
use winapi::shared::minwindef::UINT;
use winapi::um::d3dcommon::D3D_FEATURE_LEVEL;
use super::*;
use metrics::transmute_safe;

/// Driver object for ID3D11On12Device
pub struct Device(*mut ID3D11On12Device); HandleWrapper!(for Device[ID3D11On12Device] + FromRawHandle);
impl dxgi::DeviceChild for Device { fn parent(&self) -> IOResult<dxgi::Device> { self.query_interface() } }

impl Device
{
    pub fn new(device12: &d3d12::Device, queues: &[&d3d12::CommandQueue], bgra_support: bool, debug: bool)
        -> IOResult<(Self, d3d11::ImmediateContext)>
    {
        let (mut d11, mut dc) = (std::ptr::null_mut(), std::ptr::null_mut());
        let flags = if bgra_support { D3D11_CREATE_DEVICE_BGRA_SUPPORT } else { 0 } | if debug { D3D11_CREATE_DEVICE_DEBUG } else { 0 };
        let queues = queues.into_iter().map(|x| x.as_iunknown()).collect::<Vec<_>>();
        unsafe
        {
            D3D11On12CreateDevice(device12.as_iunknown(), flags, std::ptr::null(), 0, queues.as_ptr(), queues.len() as _, 0, &mut d11, &mut dc, std::ptr::null_mut())
        }.to_result_with(|| unsafe { d3d11::Device::from_raw_handle(d11) })
        .and_then(|d11| d11.query_interface::<Device>().map(|don| (don, unsafe { d3d11::ImmediateContext::from_raw_handle(dc) })))
    }
}

/// Driver object for Wrapped Resource
pub struct WrappedResource<T: d3d11::Resource>(T);
impl<T: d3d11::Resource> WrappedResource<T>
{
    pub fn as_ptr(&self) -> *mut ID3D11Resource { self.0.as_raw_resource_ptr() }
}
impl Device
{
    pub fn new_wrapped_resource<T: d3d11::Resource + Handle>(&self, source: &d3d12::Resource, bind: d3d11::BindFlags,
        acquire_state: d3d12::ResourceState, release_state: d3d12::ResourceState) -> IOResult<WrappedResource<T>>
        where T: FromRawHandle<<T as Handle>::RawType>
    {
        let mut handle = std::ptr::null_mut();
        let res_flags = D3D11_RESOURCE_FLAGS { BindFlags: *transmute_safe(&bind), .. unsafe { std::mem::zeroed() } };
        unsafe { (*self.0).CreateWrappedResource(source.as_iunknown(), &res_flags, acquire_state as _, release_state as _, &T::RawType::uuidof(), &mut handle) }
            .to_result_with(|| unsafe { WrappedResource(T::from_raw_handle(handle as _)) })
    }
    pub fn release_wrapped_resources(&self, resources: &[*mut ID3D11Resource])
    {
        unsafe { (*self.0).ReleaseWrappedResources(resources.as_ptr() as *mut _, resources.len() as _) }
    }
    pub fn acquire_wrapped_resources(&self, resources: &[*mut ID3D11Resource])
    {
        unsafe { (*self.0).AcquireWrappedResources(resources.as_ptr() as *mut _, resources.len() as _) }
    }
}
impl<T: d3d11::Resource> dxgi::SurfaceChild for WrappedResource<T> where T: dxgi::SurfaceChild
{
    fn base(&self) -> IOResult<dxgi::Surface> { self.0.base() }
}

#[link(name = "d3d11")]
extern "system"
{
    fn D3D11On12CreateDevice(pDevice: *mut IUnknown, Flags: UINT, pFeatureLevels: *const D3D_FEATURE_LEVEL, FeatureLevels: UINT,
        ppCommandQueues: *const *mut IUnknown, NumQueues: UINT, NodeMask: UINT, ppDevice: *mut *mut ID3D11Device, ppImmediateContenxt: *mut *mut ID3D11DeviceContext,
        pChosenFeatureLevel: *mut D3D_FEATURE_LEVEL) -> HRESULT;
}
