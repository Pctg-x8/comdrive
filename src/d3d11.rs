//! Direct3D11 Driver

use super::*;
use winapi::ctypes::{c_void, c_char};
use winapi::um::d3d11::*;
use winapi::um::d3dcommon::*;
use winapi::shared::dxgiformat::*;
use metrics::MarkForSameBits;
use std::ptr::{null, null_mut};
use std::mem::size_of;

pub use winapi::um::d3d11::D3D11_VIEWPORT as Viewport;
pub use winapi::um::d3dcommon::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP as TriangleStripTopo;

pub type GenericResult<T> = std::result::Result<T, Box<std::error::Error>>;

/// Driver object for ID3D11Device
pub struct Device(*mut ID3D11Device); HandleWrapper!(for Device[ID3D11Device] + FromRawHandle);
/// Driver object for ID3D11DeviceContext for Immediate Submission
pub struct ImmediateContext(*mut ID3D11DeviceContext); HandleWrapper!(for ImmediateContext[ID3D11DeviceContext] + FromRawHandle);
/// Driver object for ID3D11DeviceContext for Deferred Submission
pub struct DeferredContext(*mut ID3D11DeviceContext); HandleWrapper!(for DeferredContext[ID3D11DeviceContext] + FromRawHandle);

impl Device
{
    /// Create Device and Immediate Context
    pub fn new(adapter: Option<&dxgi::Adapter>, compatible_d2d: bool) -> IOResult<(Device, ImmediateContext)>
    {
        let flags = if compatible_d2d { D3D11_CREATE_DEVICE_BGRA_SUPPORT } else { 0 } | D3D11_CREATE_DEVICE_DEBUG;
        let (mut hdev, mut himm) = (null_mut(), null_mut());
        unsafe
        {
            D3D11CreateDevice(adapter.map(AsRawHandle::as_raw_handle).unwrap_or_else(null_mut),
                if adapter.is_some() { D3D_DRIVER_TYPE_UNKNOWN } else { D3D_DRIVER_TYPE_HARDWARE },
                null_mut(), flags, null(), 0, D3D11_SDK_VERSION, &mut hdev, null_mut(), &mut himm)
                    .to_result_with(|| (Device(hdev), ImmediateContext(himm)))
        }
    }
}
impl dxgi::DeviceChild for Device { fn parent(&self) -> IOResult<dxgi::Device> { self.query_interface() } }
impl ImmediateContext
{
    pub fn flush(&self) { unsafe { (*self.0).Flush() }; }
}

/// リソースのパイプラインへの束縛フラグ
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BindFlags(D3D11_BIND_FLAG);
impl BindFlags
{
    pub fn new() -> Self { BindFlags(0) }
    pub fn vertex_buffer(self) -> Self { BindFlags(self.0 | D3D11_BIND_VERTEX_BUFFER) }
    pub fn index_buffer(self) -> Self { BindFlags(self.0 | D3D11_BIND_INDEX_BUFFER) }
    pub fn constant_buffer(self) -> Self { BindFlags(D3D11_BIND_CONSTANT_BUFFER) }
    pub fn shader_resource(self) -> Self { BindFlags(self.0 | D3D11_BIND_SHADER_RESOURCE) }
    pub fn stream_output(self) -> Self { BindFlags(self.0 | D3D11_BIND_STREAM_OUTPUT) }
    pub fn render_target(self) -> Self { BindFlags(self.0 | D3D11_BIND_RENDER_TARGET) }
    pub fn depth_stencil(self) -> Self { BindFlags(self.0 | D3D11_BIND_DEPTH_STENCIL) }
    pub fn unordered_access(self) -> Self { BindFlags(self.0 | D3D11_BIND_UNORDERED_ACCESS) }
}
unsafe impl MarkForSameBits<D3D11_BIND_FLAG> for BindFlags {}

/// Driver object for ID3D11Texture2D
pub struct Texture2D(*mut ID3D11Texture2D); HandleWrapper!(for Texture2D[ID3D11Texture2D] + FromRawHandle);
impl dxgi::SurfaceChild for Texture2D { fn base(&self) -> IOResult<dxgi::Surface> { self.query_interface() } }
pub struct TextureDesc2D(D3D11_TEXTURE2D_DESC);
impl TextureDesc2D
{
    pub fn new(width: u32, height: u32, format: dxgi::Format) -> Self
    {
        TextureDesc2D(D3D11_TEXTURE2D_DESC
        {
            Width: width as _, Height: height as _, Format: format,
            MipLevels: 1, ArraySize: 1, SampleDesc: dxgi::SampleDesc { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT, BindFlags: 0, CPUAccessFlags: 0, MiscFlags: 0
        })
    }
    pub fn bound(&mut self, flags: BindFlags) -> &mut Self { self.0.BindFlags = flags.0; self }
    pub fn immutable(&mut self) -> &mut Self { self.0.Usage = D3D11_USAGE_IMMUTABLE; self }
    pub fn staging(&mut self) -> &mut Self { self.0.Usage = D3D11_USAGE_STAGING; self }
    pub fn cpu_readable(&mut self) -> &mut Self { self.0.CPUAccessFlags |= D3D11_CPU_ACCESS_READ; self }
    pub fn create(&self, device: &Device, init_data: Option<&[u8]>, pitch: u32) -> IOResult<Texture2D>
    {
        assert!(self.0.Usage != D3D11_USAGE_IMMUTABLE || init_data.is_some(), "Using immutable texture without initial data");
        let mut handle = std::ptr::null_mut();
        let hr = if let Some(p) = init_data
        {
            let initial_data = D3D11_SUBRESOURCE_DATA { pSysMem: p.as_ptr() as _, SysMemPitch: pitch as _, SysMemSlicePitch: p.len() as _ };
            unsafe { (*device.0).CreateTexture2D(&self.0, &initial_data, &mut handle) }
        }
        else { unsafe { (*device.0).CreateTexture2D(&self.0, std::ptr::null(), &mut handle) } };
        hr.to_result_with(|| Texture2D(handle))
    }
}

/// バッファ(GPU VRAM上のデータブロック)
pub struct Buffer(*mut ID3D11Buffer, usize); HandleWrapper!(for Buffer[ID3D11Buffer]);
impl Device
{
    /// 不変バッファの作成
    pub fn new_array_buffer<T>(&self, bind_flags: BindFlags, initial_data: &[T]) -> IOResult<Buffer>
    {
        let desc = D3D11_BUFFER_DESC
        {
            BindFlags: bind_flags.0, ByteWidth: (size_of::<T>() * initial_data.len()) as _,
            StructureByteStride: size_of::<f32>() as _, Usage: D3D11_USAGE_IMMUTABLE,
            CPUAccessFlags: 0, MiscFlags: 0
        };
        let initial_data = D3D11_SUBRESOURCE_DATA { pSysMem: initial_data.as_ptr() as _, SysMemPitch: 0, SysMemSlicePitch: 0 };
        let mut handle = null_mut();
        unsafe
        {
            (*self.0).CreateBuffer(&desc, &initial_data, &mut handle).to_result_with(|| Buffer(handle, size_of::<T>()))
        }
    }
    /// 不変バッファの作成
    pub fn new_buffer<T>(&self, bind_flags: BindFlags, initial_data: &T) -> IOResult<Buffer>
    {
        let desc = D3D11_BUFFER_DESC
        {
            BindFlags: bind_flags.0, ByteWidth: size_of::<T>() as _,
            StructureByteStride: size_of::<f32>() as _, Usage: D3D11_USAGE_IMMUTABLE,
            CPUAccessFlags: 0, MiscFlags: 0
        };
        let initial_data = D3D11_SUBRESOURCE_DATA { pSysMem: initial_data as *const _ as *const _, SysMemPitch: 0, SysMemSlicePitch: 0 };
        let mut handle = null_mut();
        unsafe
        {
            (*self.0).CreateBuffer(&desc, &initial_data, &mut handle).to_result_with(|| Buffer(handle, size_of::<T>()))
        }
    }
    /// UpdateSubresource可能なバッファの作成
    pub fn new_buffer_update(&self, bind_flags: BindFlags, size: usize) -> IOResult<Buffer>
    {
        let desc = D3D11_BUFFER_DESC
        {
            BindFlags: bind_flags.0, ByteWidth: size as _, StructureByteStride: size_of::<f32>() as _,
            Usage: D3D11_USAGE_DEFAULT, CPUAccessFlags: 0, MiscFlags: 0
        };
        let mut handle = null_mut();
        unsafe { (*self.0).CreateBuffer(&desc, null(), &mut handle).to_result_with(|| Buffer(handle, 0)) }
    }
    /// 可変バッファの作成
    pub fn new_buffer_mut(&self, bind_flags: BindFlags, size: usize) -> IOResult<Buffer>
    {
        let desc = D3D11_BUFFER_DESC
        {
            BindFlags: bind_flags.0, ByteWidth: size as _, StructureByteStride: size_of::<f32>() as _,
            Usage: D3D11_USAGE_DEFAULT, CPUAccessFlags: D3D11_CPU_ACCESS_WRITE, MiscFlags: 0
        };
        let mut handle = null_mut();
        unsafe { (*self.0).CreateBuffer(&desc, null(), &mut handle).to_result_with(|| Buffer(handle, 0)) }
    }
}

/// リソース
pub trait Resource { fn as_raw_resource_ptr(&self) -> *mut ID3D11Resource; }
impl Resource for Texture2D { fn as_raw_resource_ptr(&self) -> *mut ID3D11Resource { self.0 as _ } }
impl Resource for Buffer { fn as_raw_resource_ptr(&self) -> *mut ID3D11Resource { self.0 as _ } }

/// 入力レイアウト
pub struct InputLayout(*mut ID3D11InputLayout); HandleWrapper!(for InputLayout[ID3D11InputLayout]);
impl Device
{
    /// 入力レイアウトの作成
    pub fn new_input_layout(&self, input_elements: &[InputElement], signature: &[u8]) -> IOResult<InputLayout>
    {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateInputLayout(input_elements.as_ptr() as *const _, input_elements.len() as _,
            signature.as_ptr() as _, signature.len() as _, &mut handle) }.to_result_with(|| InputLayout(handle))
    }
}
/// 入力エレメント
pub struct InputElement(D3D11_INPUT_ELEMENT_DESC);
unsafe impl MarkForSameBits<D3D11_INPUT_ELEMENT_DESC> for InputElement {}
impl InputElement
{
    pub fn per_vertex(sem_name: *const c_char, sem_index: u32, format: dxgi::Format, input_slot: u32, byte_offset: u32) -> Self
    {
        InputElement(D3D11_INPUT_ELEMENT_DESC
        {
            SemanticName: sem_name, SemanticIndex: sem_index, Format: format, InputSlot: input_slot, AlignedByteOffset: byte_offset,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA, InstanceDataStepRate: 0
        })
    }
}

/// サンプラーステート
pub struct SamplerStateBuilder(D3D11_SAMPLER_DESC);
impl SamplerStateBuilder
{
    pub fn new() -> Self
    {
        SamplerStateBuilder(D3D11_SAMPLER_DESC
        {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP, AddressV: D3D11_TEXTURE_ADDRESS_CLAMP, AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            MipLODBias: 0.0, MaxAnisotropy: 1, ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            BorderColor: [0.0; 4], MinLOD: 0.0, MaxLOD: 0.0
        })
    }
    pub fn filter(&mut self, f: D3D11_FILTER) -> &mut Self { self.0.Filter = f; self }
    pub fn address(&mut self, u: D3D11_TEXTURE_ADDRESS_MODE, v: D3D11_TEXTURE_ADDRESS_MODE, w: D3D11_TEXTURE_ADDRESS_MODE) -> &mut Self
    {
        self.0.AddressU = u; self.0.AddressV = v; self.0.AddressW = w; self
    }
    pub fn mip_lod_bias(&mut self, bias: f32) -> &mut Self { self.0.MipLODBias = bias; self }
    pub fn max_anisotropy(&mut self, aniso: u8) -> &mut Self
    {
        assert!(1 <= aniso && aniso <= 16, "MaxAnisotropy must be between 1 and 16");
        self.0.MaxAnisotropy = aniso as _; self
    }
    pub fn cmp_func(&mut self, f: D3D11_COMPARISON_FUNC) -> &mut Self { self.0.ComparisonFunc = f; self }
    pub fn border_color(&mut self, r: f32, g: f32, b: f32, a: f32) -> &mut Self
    {
        self.0.BorderColor = [r, g, b, a]; self
    }
    pub fn lod_range(&mut self, min: f32, max: f32) -> &mut Self { self.0.MinLOD = min; self.0.MaxLOD = max; self }
    
    pub fn create(&self, device: &Device) -> IOResult<SamplerState>
    {
        let mut h = std::ptr::null_mut();
        unsafe { (*device.0).CreateSamplerState(&self.0, &mut h) }.to_result_with(|| SamplerState(h))
    }
}
/// サンプラーステート
pub struct SamplerState(*mut ID3D11SamplerState); HandleWrapper!(for SamplerState[ID3D11SamplerState]);

/// 頂点シェーダ
pub struct VertexShader(*mut ID3D11VertexShader); HandleWrapper!(for VertexShader[ID3D11VertexShader]);
/// ピクセルシェーダ
pub struct PixelShader(*mut ID3D11PixelShader); HandleWrapper!(for PixelShader[ID3D11PixelShader]);
impl Device
{
    /// 頂点シェーダの作成
    pub fn new_vertex_shader<Source: d3d::ShaderSource + ?Sized>(&self, source: &Source) -> GenericResult<VertexShader>
    {
        source.binary().map_err(From::from).and_then(|b| unsafe
        {
            let mut handle = null_mut();
            (*self.0).CreateVertexShader(b.as_ptr() as _, b.len() as _, null_mut(), &mut handle)
                .to_result_with(|| VertexShader(handle)).map_err(From::from)
        })
    }
    /// ピクセルシェーダの作成
    pub fn new_pixel_shader<Source: d3d::ShaderSource + ?Sized>(&self, source: &Source) -> GenericResult<PixelShader>
    {
        source.binary().map_err(From::from).and_then(|b| unsafe
        {
            let mut handle = null_mut();
            (*self.0).CreatePixelShader(b.as_ptr() as _, b.len() as _, null_mut(), &mut handle)
                .to_result_with(|| PixelShader(handle)).map_err(From::from)
        })
    }
}

pub trait ViewDescriptable<ViewDesc> { fn descriptor(&self) -> ViewDesc; }
impl ViewDescriptable<D3D11_RENDER_TARGET_VIEW_DESC> for Texture2D
{
    fn descriptor(&self) -> D3D11_RENDER_TARGET_VIEW_DESC
    {
        D3D11_RENDER_TARGET_VIEW_DESC
        {
            ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
            Format: DXGI_FORMAT_UNKNOWN,
            .. unsafe { std::mem::zeroed() }
        }
    }
}

/// レンダーターゲットビュー
pub struct RenderTargetView(*mut ID3D11RenderTargetView); HandleWrapper!(for RenderTargetView[ID3D11RenderTargetView]);
impl Device
{
    /// レンダーターゲットビューをつくる！
    pub fn new_render_target_view<R: ViewDescriptable<D3D11_RENDER_TARGET_VIEW_DESC> + Resource>(&self, resource: &R) -> IOResult<RenderTargetView>
    {
        let mut handle = null_mut();
        unsafe
        {
            (*self.0).CreateRenderTargetView(resource.as_raw_resource_ptr(), &resource.descriptor(), &mut handle)
                .to_result_with(|| RenderTargetView(handle))
        }
    }
}
/// 深度ステンシルビュー(未完成)
pub struct DepthStencilView(*mut ID3D11DepthStencilView); HandleWrapper!(for DepthStencilView[ID3D11DepthStencilView]);
/// シェーダリソースビュー
pub struct ShaderResourceView(*mut ID3D11ShaderResourceView); HandleWrapper!(for ShaderResourceView[ID3D11ShaderResourceView]);
impl Device
{
    /// シェーダリソースビューを作る
    pub fn new_shader_resource_view<R: Resource>(&self, resource: &R) -> IOResult<ShaderResourceView>
    {
        let mut handle = null_mut();
        unsafe
        {
            (*self.0).CreateShaderResourceView(resource.as_raw_resource_ptr(), null(), &mut handle)
                .to_result_with(|| ShaderResourceView(handle))
        }
    }
}

/// コマンドたち
impl ImmediateContext
{
    /// レンダーターゲットビューのクリア
    pub fn clear_rtv(&self, target: &RenderTargetView, rgba: &[f32; 4]) -> &Self
    {
        unsafe { (*self.0).ClearRenderTargetView(target.0, rgba) };
        self
    }
    /// レンダーターゲットの設定
    pub fn set_render_targets(&self, targets: &[*mut ID3D11RenderTargetView], depth: Option<&DepthStencilView>) -> &Self
    {
        unsafe { (*self.0).OMSetRenderTargets(targets.len() as _, targets.as_ptr(), depth.map(|x| x.0).unwrap_or(std::ptr::null_mut())) };
        self
    }
    /// ビューポートのこうしん
    pub fn set_viewports(&self, viewports: &[D3D11_VIEWPORT]) -> &Self
    {
        unsafe { (*self.0).RSSetViewports(viewports.len() as _, viewports.as_ptr()) };
        self
    }
    /// 頂点シェーダの設定
    pub fn set_vertex_shader(&self, shader: &VertexShader) -> &Self
    {
        unsafe { (*self.0).VSSetShader(shader.0, std::ptr::null(), 0) };
        self
    }
    /// ピクセルシェーダの設定
    pub fn set_pixel_shader(&self, shader: &PixelShader) -> &Self
    {
        unsafe { (*self.0).PSSetShader(shader.0, std::ptr::null(), 0) };
        self
    }
    /// 頂点シェーダの定数バッファを設定
    pub fn set_vertex_constant_buffers(&self, buffers: &[*mut ID3D11Buffer]) -> &Self
    {
        unsafe { (*self.0).VSSetConstantBuffers(0, buffers.len() as _, buffers.as_ptr()) };
        self
    }
    /// ピクセルシェーダの定数バッファを設定
    pub fn set_pixel_constant_buffers(&self, buffers: &[*mut ID3D11Buffer]) -> &Self
    {
        unsafe { (*self.0).PSSetConstantBuffers(0, buffers.len() as _, buffers.as_ptr()) };
        self
    }
    /// ピクセルシェーダのリソースを設定
    pub fn set_pixel_resource_views(&self, views: &[*mut ID3D11ShaderResourceView]) -> &Self
    {
        unsafe { (*self.0).PSSetShaderResources(0, views.len() as _, views.as_ptr()) }; self
    }
    /// ピクセルシェーダのサンプラーを設定
    pub fn set_pixel_samplers(&self, samplers: &[*mut ID3D11SamplerState]) -> &Self
    {
        unsafe { (*self.0).PSSetSamplers(0, samplers.len() as _, samplers.as_ptr()) }; self
    }
    /// プリミティブトポロジの設定
    pub fn set_primitive_topology(&self, topo: D3D11_PRIMITIVE_TOPOLOGY) -> &Self
    {
        unsafe { (*self.0).IASetPrimitiveTopology(topo) }; self
    }
    /// 入力レイアウトの設定
    pub fn set_input_layout(&self, layout: &InputLayout) -> &Self
    {
        unsafe { (*self.0).IASetInputLayout(layout.0) }; self
    }
    /// 頂点バッファの設定
    pub fn set_vertex_buffers(&self, buffers: &[&Buffer]) -> &Self
    {
        let (bptr, strides): (Vec<_>, Vec<_>) = buffers.iter().map(|&&Buffer(p, s)| (p, s as u32)).unzip();
        let offsets = ReturnIterator::new(0).cycle().take(buffers.len()).collect::<Vec<_>>();
        unsafe { (*self.0).IASetVertexBuffers(0, bptr.len() as _, bptr.as_ptr(), strides.as_ptr(), offsets.as_ptr()) };
        self
    }
    /// かく！
    pub fn draw(&self, vertex_count: u32) -> &Self
    {
        unsafe { (*self.0).Draw(vertex_count, 0) }; self
    }
    /// サブリソースのこうしん
    pub fn update_subresource<R: Resource>(&self, buffer: &R, data: *const c_void) -> &Self
    {
        unsafe { (*self.0).UpdateSubresource(buffer.as_raw_resource_ptr(), 0, std::ptr::null(), data, 0, 0) };
        self
    }
    /// リソースのコピー
    pub fn copy_resource<Res1: Resource, Res2: Resource>(&self, src: &Res1, dst: &Res2) -> &Self
    {
        unsafe{ (*self.0).CopyResource(dst.as_raw_resource_ptr(), src.as_raw_resource_ptr()) }; self
    }
}

pub use winapi::um::d3d11::{
    D3D11_MAP_READ as MAP_READ, D3D11_MAP_WRITE as MAP_WRITE, D3D11_MAP_READ_WRITE as MAP_RW,
    D3D11_MAP_WRITE_DISCARD as MAP_WRITE_DISCARD, D3D11_MAP_WRITE_NO_OVERWRITE as MAP_WRITE_NO_OVERWRITE
};

/// マップ済みリソース
pub struct MappedResource<'c, 'r, R: Resource + 'r>(&'c ImmediateContext, &'r R, D3D11_MAPPED_SUBRESOURCE);
impl ImmediateContext
{
    /// リソースをシステムメモリにマップ
    pub fn map<'c, 'r, R: Resource + 'r>(&'c self, res: &'r R, rwmode: D3D11_MAP) -> IOResult<MappedResource<'c, 'r, R>>
    {
        let mut mp = unsafe { std::mem::uninitialized() };
        unsafe { (*self.0).Map(res.as_raw_resource_ptr(), 0, rwmode, 0, &mut mp) }.to_result_with(|| MappedResource(self, res, mp))
    }
}
impl<'c, 'r, R: Resource + 'r> Drop for MappedResource<'c, 'r, R>
{
    /// リソースをアンマップ
    fn drop(&mut self)
    {
        unsafe { (*self.0 .0).Unmap(self.1.as_raw_resource_ptr(), 0); }
    }
}
impl<'c, 'r, R: Resource + 'r> MappedResource<'c, 'r, R>
{
    pub fn data_ptr(&self) -> *const c_void { self.2.pData }
    pub fn data_ptr_mut(&self) -> *mut c_void { self.2.pData }
    pub fn row_pitch(&self) -> usize { self.2.RowPitch as _ }
    pub fn depth_pitch(&self) -> usize { self.2.DepthPitch as _ }
}

/// Iterator Support
pub struct ReturnIterator<T>(Option<T>);
impl<T: Clone> Clone for ReturnIterator<T>
{
    fn clone(&self) -> Self { ReturnIterator(self.0.clone()) }
}
impl<T> std::iter::Iterator for ReturnIterator<T>
{
    type Item = T;
    fn next(&mut self) -> Option<T> { self.0.take() }
}
impl<T> ReturnIterator<T>
{
    fn new(v: T) -> Self { ReturnIterator(Some(v)) }
}
