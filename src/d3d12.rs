//! D3D12 Driver

use super::*;
use metrics::*;
use std::mem::size_of;
use std::ops::Deref;
use std::path::Path;
use winapi::ctypes::c_void;
use winapi::shared::dxgiformat::*;
use winapi::shared::dxgitype::*;
use winapi::shared::guiddef::REFIID;
use winapi::shared::ntdef::HANDLE;
use winapi::um::d3d12::*;
use winapi::um::d3d12sdklayers::*;
use winapi::um::d3dcommon::*;
use winapi::um::d3dcompiler::{D3DGetBlobPart, D3D_BLOB_ROOT_SIGNATURE};

pub use winapi::um::d3d12::D3D12_DEFAULT_SAMPLE_MASK as DefaultSampleMask;
pub use winapi::um::d3d12::D3D12_GRAPHICS_PIPELINE_STATE_DESC as GraphicsPipelineStateDesc;
pub use winapi::um::d3d12::D3D12_INDEX_BUFFER_VIEW as IndexBufferView;
pub use winapi::um::d3d12::D3D12_INPUT_ELEMENT_DESC as InputElementDesc;
pub use winapi::um::d3d12::D3D12_RANGE as Range;
pub use winapi::um::d3d12::D3D12_RASTERIZER_DESC as RasterizerDesc;
pub use winapi::um::d3d12::D3D12_RESOURCE_ALLOCATION_INFO as ResourceAllocationInfo;
pub use winapi::um::d3d12::D3D12_SHADER_BYTECODE as ShaderBytecode;
pub use winapi::um::d3d12::D3D12_VERTEX_BUFFER_VIEW as VertexBufferView;
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum FillMode {
    Solid = D3D12_FILL_MODE_SOLID as _,
    Wired = D3D12_FILL_MODE_WIREFRAME as _,
}
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum CullMode {
    Front = D3D12_CULL_MODE_FRONT as _,
    Back = D3D12_CULL_MODE_BACK as _,
    None = D3D12_CULL_MODE_NONE as _,
}
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum PrimitiveTopologyType {
    Point = D3D12_PRIMITIVE_TOPOLOGY_TYPE_POINT as _,
    Line = D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE as _,
    Triangle = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE as _,
    Patch = D3D12_PRIMITIVE_TOPOLOGY_TYPE_PATCH as _,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    PointList = D3D_PRIMITIVE_TOPOLOGY_POINTLIST as _,
    LineList = D3D_PRIMITIVE_TOPOLOGY_LINELIST as _,
    LineStrip = D3D_PRIMITIVE_TOPOLOGY_LINESTRIP as _,
    LineListWithAdjacency = D3D_PRIMITIVE_TOPOLOGY_LINELIST_ADJ as _,
    LineStripWithAdjacency = D3D_PRIMITIVE_TOPOLOGY_LINESTRIP_ADJ as _,
    TriangleList = D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST as _,
    TriangleStrip = D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP as _,
    TriangleListWithAdjacency = D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST_ADJ as _,
    TriangleStripWithAdjacency = D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP_ADJ as _,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputClassification {
    PerVertex = D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA as _,
    PerInstance = D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA as _,
}

/// Driver object for ID2D1Device
#[repr(transparent)]
pub struct Device(*mut ID3D12Device);
HandleWrapper!(for Device[ID3D12Device] + FromRawHandle);
impl Device {
    /// デバッグレイヤーを有効化
    pub fn enable_debug_layer() -> IOResult<()> {
        let mut dbg = std::ptr::null_mut();
        unsafe {
            D3D12GetDebugInterface(&ID3D12Debug::uuidof(), &mut dbg)
                .to_result_with(|| dbg as *mut ID3D12Debug)
                .map(|dbg| (*dbg).EnableDebugLayer())
        }
    }

    /// Create
    pub fn new<Adapter: AsIUnknown + ?Sized>(
        adapter: &Adapter,
        min_feature_level: d3d::FeatureLevel,
    ) -> IOResult<Self> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            D3D12CreateDevice(
                adapter.as_iunknown(),
                min_feature_level as D3D_FEATURE_LEVEL,
                &ID3D12Device::uuidof(),
                &mut handle,
            )
            .to_result_with(|| Device(handle as _))
        }
    }

    /// レンダーターゲットビューの作成
    pub fn create_render_target_view(
        &self,
        res: &Resource,
        desc: Option<&D3D12_RENDER_TARGET_VIEW_DESC>,
        handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    ) {
        unsafe {
            (*self.0).CreateRenderTargetView(
                res.0,
                desc.map(|x| x as _).unwrap_or(std::ptr::null()),
                handle,
            )
        };
    }
    /// シェーダリソースビューの作成
    pub fn create_shader_resource_view(
        &self,
        res: &Resource,
        desc: Option<&D3D12_SHADER_RESOURCE_VIEW_DESC>,
        handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    ) {
        unsafe {
            (*self.0).CreateShaderResourceView(
                res.0,
                desc.map(|x| x as _).unwrap_or(std::ptr::null()),
                handle,
            )
        };
    }

    /// リソースのコピーに関する情報を入手
    /// (CopyableFootprintのIterator, TotalBytes)の順で帰る
    pub fn get_copyable_footprints(
        &self,
        rd: &D3D12_RESOURCE_DESC,
        subresource_range: std::ops::Range<u32>,
        base_offset: u64,
    ) -> (CopyableFootprintsIterator, u64) {
        let subresource_count = subresource_range.len();
        let mut layouts = Vec::with_capacity(subresource_count);
        unsafe {
            layouts.set_len(subresource_count);
        }
        let mut row_counts = Vec::with_capacity(subresource_count);
        unsafe {
            row_counts.set_len(subresource_count);
        }
        let mut row_sizes = Vec::with_capacity(subresource_count);
        unsafe {
            row_sizes.set_len(subresource_count);
        }
        let mut total_bytes: u64 = 0;

        unsafe {
            (*self.0).GetCopyableFootprints(
                rd,
                subresource_range.start,
                subresource_count as _,
                base_offset,
                layouts.as_mut_ptr(),
                row_counts.as_mut_ptr(),
                row_sizes.as_mut_ptr(),
                &mut total_bytes,
            );
        }
        (
            CopyableFootprintsIterator(
                layouts
                    .into_iter()
                    .zip(row_counts.into_iter())
                    .zip(row_sizes.into_iter()),
            ),
            total_bytes,
        )
    }

    /// 共有用Windowsハンドルの作成
    pub fn create_shared_handle<R: AsRawHandle<ID3D12DeviceChild>>(
        &self,
        r: &R,
        security_attributes: Option<&winapi::um::minwinbase::SECURITY_ATTRIBUTES>,
        name: &widestring::WideCString,
    ) -> IOResult<HANDLE> {
        let mut h = std::ptr::null_mut();

        unsafe {
            (*self.0)
                .CreateSharedHandle(
                    r.as_raw_handle(),
                    security_attributes.map_or_else(std::ptr::null, |p| p as *const _),
                    winapi::um::winnt::GENERIC_ALL,
                    name.as_ptr(),
                    &mut h,
                )
                .to_result_with(move || h)
        }
    }
}
unsafe impl Sync for Device {}
unsafe impl Send for Device {}

/// リソースのコピーに関する情報
pub struct CopyableFootprint {
    pub placed_footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    pub row_count: u32,
    pub row_size_in_bytes: u64,
}
/// リソースのコピーに関する情報を整理して取り出すためのもの
pub struct CopyableFootprintsIterator(
    std::iter::Zip<
        std::iter::Zip<
            std::vec::IntoIter<D3D12_PLACED_SUBRESOURCE_FOOTPRINT>,
            std::vec::IntoIter<u32>,
        >,
        std::vec::IntoIter<u64>,
    >,
);
impl Iterator for CopyableFootprintsIterator {
    type Item = CopyableFootprint;

    fn next(&mut self) -> Option<CopyableFootprint> {
        let ((f, rc), rs) = self.0.next()?;

        Some(CopyableFootprint {
            placed_footprint: f,
            row_count: rc,
            row_size_in_bytes: rs,
        })
    }
}

/// コマンドバッファ/キューのタイプ
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum CommandType {
    Direct = D3D12_COMMAND_LIST_TYPE_DIRECT,
    Bundle = D3D12_COMMAND_LIST_TYPE_BUNDLE,
    Compute = D3D12_COMMAND_LIST_TYPE_COMPUTE,
    Copy = D3D12_COMMAND_LIST_TYPE_COPY,
}
/// コマンドキュー
#[repr(transparent)]
pub struct CommandQueue(*mut ID3D12CommandQueue);
HandleWrapper!(for CommandQueue[ID3D12CommandQueue] + FromRawHandle);
impl Device {
    /// コマンドキューの作成
    pub fn new_command_queue(
        &self,
        cmd_type: CommandType,
        priority: i32,
    ) -> IOResult<CommandQueue> {
        let desc = D3D12_COMMAND_QUEUE_DESC {
            Type: cmd_type as _,
            Priority: priority,
            Flags: 0,
            NodeMask: 0,
        };
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.0)
                .CreateCommandQueue(&desc, &ID3D12CommandQueue::uuidof(), &mut handle)
                .to_result_with(|| CommandQueue(handle as _))
        }
    }
}
impl CommandQueue {
    /// コマンドバッファの実行
    pub fn execute(&mut self, buffers: &[*mut ID3D12CommandList]) -> &Self {
        unsafe { (*self.0).ExecuteCommandLists(buffers.len() as _, buffers.as_ptr() as *mut _) };
        self
    }
    /// GPUからフェンスをアップデートするように指示
    pub fn signal(&mut self, fence: &Fence, value: u64) -> IOResult<&Self> {
        unsafe { (*self.0).Signal(fence.0, value).to_result(self) }
    }
    // GPUでフェンスを待つように指示
    pub fn wait(&mut self, fence: &Fence, value: u64) -> IOResult<&Self> {
        unsafe { (*self.0).Wait(fence.0, value).to_result(self) }
    }
}
unsafe impl Sync for CommandQueue {}
unsafe impl Send for CommandQueue {}

/// コマンドアロケータ
pub struct CommandAllocator(*mut ID3D12CommandAllocator, CommandType);
HandleWrapper!(for CommandAllocator[ID3D12CommandAllocator]);
impl Device {
    /// コマンドアロケータの作成
    pub fn new_command_allocator(&self, cmd_type: CommandType) -> IOResult<CommandAllocator> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.0)
                .CreateCommandAllocator(
                    cmd_type as _,
                    &ID3D12CommandAllocator::uuidof(),
                    &mut handle,
                )
                .to_result_with(|| CommandAllocator(handle as _, cmd_type))
        }
    }
}
impl CommandAllocator {
    /// リセット
    pub fn reset(&mut self) -> IOResult<()> {
        unsafe { (*self.0).Reset().checked() }
    }
}
unsafe impl Sync for CommandAllocator {}
unsafe impl Send for CommandAllocator {}

/// デスクリプタヒープの中身
#[repr(u32)]
#[derive(Clone, Copy)]
pub enum DescriptorHeapContents {
    RenderTargetViews = D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
    ShaderViews = D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
    Samplers = D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
    DepthStencilViews = D3D12_DESCRIPTOR_HEAP_TYPE_DSV,
}
/// デスクリプタヒープ
pub struct DescriptorHeap(*mut ID3D12DescriptorHeap, usize);
HandleWrapper!(for DescriptorHeap[ID3D12DescriptorHeap]);
impl Device {
    /// デスクリプタヒープの作成
    pub fn new_descriptor_heap(
        &self,
        contents: DescriptorHeapContents,
        count: usize,
        shader_visibility: bool,
    ) -> IOResult<DescriptorHeap> {
        let desc = D3D12_DESCRIPTOR_HEAP_DESC {
            Type: contents as _,
            NumDescriptors: count as _,
            NodeMask: 0,
            Flags: if shader_visibility {
                D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE
            } else {
                0
            },
        };
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.0).CreateDescriptorHeap(&desc, &ID3D12DescriptorHeap::uuidof(), &mut handle)
        }
        .to_result_with(|| unsafe {
            let interval = (*self.0).GetDescriptorHandleIncrementSize(contents as _);
            // let mut cpu_handle = std::mem::uninitialized();
            // (*(handle as *mut ID3D12DescriptorHeap)).GetCPUDescriptorHandleForHeapStart(&mut cpu_handle);
            DescriptorHeap(handle as _, interval as _)
        })
    }
}
impl DescriptorHeap {
    /// CPUハンドルを取得
    pub fn host_descriptor_handle_base(&self) -> HostDescriptorHandle {
        unsafe { HostDescriptorHandle((*self.0).GetCPUDescriptorHandleForHeapStart(), self.1) }
    }
    /// GPUハンドルを取得
    pub fn device_descriptor_handle_base(&self) -> DeviceDescriptorHandle {
        unsafe { DeviceDescriptorHandle((*self.0).GetGPUDescriptorHandleForHeapStart(), self.1) }
    }
}
unsafe impl Sync for DescriptorHeap {}
unsafe impl Send for DescriptorHeap {}

/// デスクリプタハンドル(CPU)
#[derive(Clone)]
pub struct HostDescriptorHandle(D3D12_CPU_DESCRIPTOR_HANDLE, usize);
/// デスクリプタハンドル(GPU)
#[derive(Clone)]
pub struct DeviceDescriptorHandle(D3D12_GPU_DESCRIPTOR_HANDLE, usize);
impl AsRef<D3D12_CPU_DESCRIPTOR_HANDLE> for HostDescriptorHandle {
    fn as_ref(&self) -> &D3D12_CPU_DESCRIPTOR_HANDLE {
        &self.0
    }
}
impl AsRef<D3D12_GPU_DESCRIPTOR_HANDLE> for DeviceDescriptorHandle {
    fn as_ref(&self) -> &D3D12_GPU_DESCRIPTOR_HANDLE {
        &self.0
    }
}
impl From<HostDescriptorHandle> for D3D12_CPU_DESCRIPTOR_HANDLE {
    fn from(v: HostDescriptorHandle) -> Self {
        v.0
    }
}
impl From<DeviceDescriptorHandle> for D3D12_GPU_DESCRIPTOR_HANDLE {
    fn from(v: DeviceDescriptorHandle) -> Self {
        v.0
    }
}
impl HostDescriptorHandle {
    /// count番目を参照
    pub fn offset(&self, count: usize) -> Self {
        HostDescriptorHandle(
            D3D12_CPU_DESCRIPTOR_HANDLE {
                ptr: self.0.ptr + (count * self.1) as usize,
            },
            self.1,
        )
    }
}
impl DeviceDescriptorHandle {
    /// count番目を参照
    pub fn offset(&self, count: usize) -> Self {
        DeviceDescriptorHandle(
            D3D12_GPU_DESCRIPTOR_HANDLE {
                ptr: self.0.ptr + (count * self.1) as u64,
            },
            self.1,
        )
    }
}

/// リソース生成フラグ
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
#[repr(transparent)]
pub struct ResourceFlag(D3D12_RESOURCE_FLAGS);
unsafe impl MarkForSameBits<D3D12_RESOURCE_FLAGS> for ResourceFlag {}
impl ResourceFlag {
    pub fn new() -> Self {
        ResourceFlag(0)
    }
    pub fn allow_render_target(&self) -> Self {
        ResourceFlag(self.0 | D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET)
    }
    pub fn allow_depth_stencil(&self) -> Self {
        ResourceFlag(self.0 | D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL)
    }
    pub fn allow_unordered_access(&self) -> Self {
        ResourceFlag(self.0 | D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS)
    }
    pub fn deny_shader_resource(&self) -> Self {
        ResourceFlag(self.0 | D3D12_RESOURCE_FLAG_DENY_SHADER_RESOURCE)
    }
    pub fn allow_cross_adapter(&self) -> Self {
        ResourceFlag(self.0 | D3D12_RESOURCE_FLAG_ALLOW_CROSS_ADAPTER)
    }
    pub fn simultaneous_access(&self) -> Self {
        ResourceFlag(self.0 | D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS)
    }
}
/// リソースの詳細
#[repr(transparent)]
pub struct ResourceDesc(D3D12_RESOURCE_DESC);
unsafe impl MarkForSameBits<D3D12_RESOURCE_DESC> for ResourceDesc {}
impl AsRef<D3D12_RESOURCE_DESC> for ResourceDesc {
    fn as_ref(&self) -> &D3D12_RESOURCE_DESC {
        &self.0
    }
}
impl From<ResourceDesc> for D3D12_RESOURCE_DESC {
    fn from(v: ResourceDesc) -> Self {
        v.0
    }
}
impl ResourceDesc {
    /// バッファ
    pub fn buffer(bytesize: usize) -> Self {
        ResourceDesc(D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Alignment: D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as _,
            Width: bytesize as _,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_UNKNOWN,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            Flags: 0,
        })
    }
    /// 平面テクスチャ
    pub fn texture2d(size: &Size2U, format: dxgi::Format) -> Self {
        ResourceDesc(D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as _,
            Width: size.width() as _,
            Height: size.height() as _,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: 0,
        })
    }

    /// テクスチャレイアウト指定(Layout)
    pub fn layout(mut self, layout: D3D12_TEXTURE_LAYOUT) -> Self {
        self.0.Layout = layout;
        self
    }
    /// リソースフラグ指定(Flags)
    pub fn flags(mut self, flags: ResourceFlag) -> Self {
        self.0.Flags = flags.0;
        self
    }
    /// サンプリング指定(SampleDesc)
    pub fn sample_desc(mut self, desc: DXGI_SAMPLE_DESC) -> Self {
        self.0.SampleDesc = desc;
        self
    }
    /// 深度指定(DepthOrArraySize)
    pub fn depth(mut self, depth: u16) -> Self {
        self.0.DepthOrArraySize = depth;
        self
    }
    /// 配列要素数指定(DepthOrArraySize)
    pub fn array_size(mut self, size: u16) -> Self {
        self.0.DepthOrArraySize = size;
        self
    }
    /// ミップマップレベル指定(MipLevels)
    pub fn mip_levels(mut self, levels: u16) -> Self {
        self.0.MipLevels = levels;
        self
    }
    /// アラインメント指定(Alignment)
    pub fn alignment(mut self, align: u64) -> Self {
        self.0.Alignment = align;
        self
    }
}
/// リソースの状態
#[repr(u32)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResourceState {
    Present = D3D12_RESOURCE_STATE_PRESENT,
    VertexAndConstantBuffer = D3D12_RESOURCE_STATE_VERTEX_AND_CONSTANT_BUFFER,
    IndexBuffer = D3D12_RESOURCE_STATE_INDEX_BUFFER,
    RenderTarget = D3D12_RESOURCE_STATE_RENDER_TARGET,
    UnorderedAccess = D3D12_RESOURCE_STATE_UNORDERED_ACCESS,
    DepthWrite = D3D12_RESOURCE_STATE_DEPTH_WRITE,
    DepthRead = D3D12_RESOURCE_STATE_DEPTH_READ,
    NonPixelShaderResource = D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE,
    PixelShaderResource = D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    StreamOut = D3D12_RESOURCE_STATE_STREAM_OUT,
    IndirectArgument = D3D12_RESOURCE_STATE_INDIRECT_ARGUMENT,
    CopyDest = D3D12_RESOURCE_STATE_COPY_DEST,
    CopySource = D3D12_RESOURCE_STATE_COPY_SOURCE,
    ResolveDest = D3D12_RESOURCE_STATE_RESOLVE_DEST,
    ResolveSource = D3D12_RESOURCE_STATE_RESOLVE_SOURCE,
    GenericRead = D3D12_RESOURCE_STATE_GENERIC_READ,
}
#[derive(Clone)]
/// クリア値
pub enum OptimizedClearValue {
    Color(DXGI_FORMAT, f32, f32, f32, f32),
    DepthStencil(DXGI_FORMAT, f32, u8),
}
impl From<OptimizedClearValue> for D3D12_CLEAR_VALUE {
    fn from(v: OptimizedClearValue) -> Self {
        match v {
            OptimizedClearValue::Color(fmt, r, g, b, a) => D3D12_CLEAR_VALUE {
                Format: fmt,
                u: unsafe { std::mem::transmute([r, g, b, a]) },
            },
            OptimizedClearValue::DepthStencil(fmt, d, s) => {
                let mut cv = D3D12_CLEAR_VALUE {
                    Format: fmt,
                    u: unsafe { std::mem::MaybeUninit::zeroed().assume_init() },
                };
                unsafe {
                    *cv.u.DepthStencil_mut() = D3D12_DEPTH_STENCIL_VALUE {
                        Depth: d,
                        Stencil: s,
                    };
                }
                cv
            }
        }
    }
}
/// リソースハンドル
#[repr(transparent)]
pub struct Resource(*mut ID3D12Resource);
HandleWrapper!(for Resource[ID3D12Resource] + FromRawHandle);
impl Device {
    /// 該当リソースを割り当てるために必要なメモリのサイズとアラインメントを返す
    pub fn get_resource_allocation_info(&self, desc: &[ResourceDesc]) -> ResourceAllocationInfo {
        unsafe { (*self.0).GetResourceAllocationInfo(0, desc.len() as _, desc.as_ptr() as _) }
    }
    /// コミット済みリソースの作成
    pub fn new_resource_committed(
        &self,
        heap_props: &HeapProperty,
        desc: &ResourceDesc,
        initial_state: ResourceState,
        clear_value: Option<&OptimizedClearValue>,
    ) -> IOResult<Resource> {
        let opt_cv = clear_value.map(|cv| cv.clone().into());
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.0).CreateCommittedResource(
                heap_props.as_ref(),
                D3D12_HEAP_FLAG_NONE,
                desc.as_ref(),
                initial_state as _,
                opt_cv
                    .as_ref()
                    .map(|x| x as *const _)
                    .unwrap_or(std::ptr::null()),
                &ID3D12Resource::uuidof(),
                &mut handle,
            )
        }
        .to_result_with(|| Resource(handle as _))
    }
    /// 予約済みリソースの作成
    pub fn new_resource_reserved(
        &self,
        desc: &D3D12_RESOURCE_DESC,
        initial_state: D3D12_RESOURCE_STATES,
        clear_value: Option<&D3D12_CLEAR_VALUE>,
    ) -> IOResult<Resource> {
        let mut h = std::ptr::null_mut();
        unsafe {
            (*self.0).CreateReservedResource(
                desc,
                initial_state,
                clear_value.map_or(std::ptr::null(), |p| p as _),
                &ID3D12Resource::uuidof(),
                &mut h,
            )
        }
        .to_result_with(|| Resource(h as _))
    }
}
impl Resource {
    /// メモリをマップする
    pub fn map<R: MappingRange>(&mut self, range: R) -> IOResult<*mut c_void> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            (*self.0).Map(
                0,
                range
                    .into_range_object()
                    .as_ref()
                    .map(|x| x as _)
                    .unwrap_or(std::ptr::null()),
                &mut ptr,
            )
        }
        .to_result_with(|| ptr)
    }
    /// メモリのマッピングを解除する
    pub fn unmap<R: MappingRange>(&mut self, range: R) {
        unsafe {
            (*self.0).Unmap(
                0,
                range
                    .into_range_object()
                    .as_ref()
                    .map(|x| x as _)
                    .unwrap_or(std::ptr::null()),
            )
        }
    }
    /// GPU内仮想アドレスを取得
    pub fn gpu_virtual_address(&self) -> GraphicsVirtualPtr {
        GraphicsVirtualPtr(unsafe { (*self.0).GetGPUVirtualAddress() })
    }

    /// 強制リリース
    pub fn release(&mut self) {
        if self.is_available() {
            unsafe { (*self.0).Release() };
            self.0 = std::ptr::null_mut();
        }
    }
    /// 空のリソースを表す
    pub unsafe fn empty() -> Self {
        Resource(std::ptr::null_mut())
    }
    /// 使用可能かどうかを返す
    pub fn is_available(&self) -> bool {
        !self.0.is_null()
    }
}
unsafe impl AsRawHandle<ID3D12DeviceChild> for Resource {
    fn as_raw_handle(&self) -> *mut ID3D12DeviceChild {
        self.0 as _
    }
}
unsafe impl Sync for Resource {}
unsafe impl Send for Resource {}

/// ヒープのプロパティ
#[repr(transparent)]
pub struct HeapProperty(D3D12_HEAP_PROPERTIES);
unsafe impl MarkForSameBits<D3D12_HEAP_PROPERTIES> for HeapProperty {}
impl AsRef<D3D12_HEAP_PROPERTIES> for HeapProperty {
    fn as_ref(&self) -> &D3D12_HEAP_PROPERTIES {
        &self.0
    }
}
impl HeapProperty {
    /// デフォルトヒープ(CPUアクセスなし)
    pub fn default() -> Self {
        HeapProperty(D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
            MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
            CreationNodeMask: 0,
            VisibleNodeMask: 0,
        })
    }
    /// アップロードヒープ(CPU書きこみ可能)
    pub fn upload() -> Self {
        HeapProperty(D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_UPLOAD,
            CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
            MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
            CreationNodeMask: 0,
            VisibleNodeMask: 0,
        })
    }
}

/// ヒープオブジェクト(リソースをまとめる)
pub struct Heap(*mut ID3D12Heap, *mut ID3D12Device);
HandleWrapper!(for Heap[ID3D12Heap]);
impl Device {
    /// ヒープの作成
    pub fn new_heap(
        &self,
        property: &HeapProperty,
        size: usize,
        flags: D3D12_HEAP_FLAGS,
    ) -> IOResult<Heap> {
        let desc = D3D12_HEAP_DESC {
            SizeInBytes: size as _,
            Properties: property.0,
            Alignment: D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as _,
            Flags: flags,
        };
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.0)
                .CreateHeap(&desc, &ID3D12Heap::uuidof(), &mut handle)
                .to_result_with(|| Heap(handle as _, self.0))
        }
    }
}
impl Heap {
    /// リソースを配置newする
    pub fn place_resource(
        &mut self,
        offset: usize,
        resource: &D3D12_RESOURCE_DESC,
        initial_state: D3D12_RESOURCE_STATES,
    ) -> IOResult<Resource> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.1)
                .CreatePlacedResource(
                    self.0,
                    offset as _,
                    resource,
                    initial_state,
                    std::ptr::null(),
                    &ID3D12Resource::uuidof(),
                    &mut handle,
                )
                .to_result_with(|| Resource(handle as _))
        }
    }
    /// バッファを配置newする
    pub fn place_buffer(
        &mut self,
        range: std::ops::Range<usize>,
        initial_state: D3D12_RESOURCE_STATES,
    ) -> IOResult<Resource> {
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
            Alignment: D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as _,
            Width: (range.end - range.start) as _,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_UNKNOWN,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
            Flags: 0,
        };

        self.place_resource(range.start, &desc, initial_state)
    }
}
unsafe impl Sync for Heap {}
unsafe impl Send for Heap {}

/// マッピングはんいを表す
pub trait MappingRange: Sized {
    fn into_range_object(self) -> Option<Range>;
}
/// はんい指定
impl MappingRange for std::ops::Range<usize> {
    fn into_range_object(self) -> Option<Range> {
        Some(Range {
            Begin: self.start as _,
            End: self.end as _,
        })
    }
}
/// 最初を省略(0と同じ)
impl MappingRange for std::ops::RangeTo<usize> {
    fn into_range_object(self) -> Option<Range> {
        Some(Range {
            Begin: 0,
            End: self.end as _,
        })
    }
}
/// 全体
impl MappingRange for std::ops::RangeFull {
    fn into_range_object(self) -> Option<Range> {
        None
    }
}
/// アプリケーションカスタム
impl MappingRange for Option<Range> {
    fn into_range_object(self) -> Option<Range> {
        self
    }
}

/// ルートシグネチャのパラメータ(シェーダ定数に関わる)
#[repr(transparent)]
pub struct RootParameter(D3D12_ROOT_PARAMETER);
unsafe impl MarkForSameBits<D3D12_ROOT_PARAMETER> for RootParameter {}
impl AsRef<D3D12_ROOT_PARAMETER> for RootParameter {
    fn as_ref(&self) -> &D3D12_ROOT_PARAMETER {
        &self.0
    }
}
impl RootParameter {
    /// 定数パラメータ
    pub fn constant(
        visibility: D3D12_SHADER_VISIBILITY,
        count: usize,
        register_index: u32,
        register_space: u32,
    ) -> Self {
        RootParameter(D3D12_ROOT_PARAMETER {
            ParameterType: D3D12_ROOT_PARAMETER_TYPE_32BIT_CONSTANTS,
            ShaderVisibility: visibility,
            u: unsafe {
                *std::mem::transmute::<_, &_>(&D3D12_ROOT_CONSTANTS {
                    Num32BitValues: count as _,
                    RegisterSpace: register_space,
                    ShaderRegister: register_index,
                })
            },
        })
    }
    /// シェーダリソースパラメータ
    pub fn shader_resource(
        visibility: D3D12_SHADER_VISIBILITY,
        register_index: u32,
        register_space: u32,
    ) -> Self {
        RootParameter(D3D12_ROOT_PARAMETER {
            ParameterType: D3D12_ROOT_PARAMETER_TYPE_SRV,
            ShaderVisibility: visibility,
            u: unsafe {
                *std::mem::transmute::<_, &_>(&D3D12_ROOT_DESCRIPTOR {
                    RegisterSpace: register_space,
                    ShaderRegister: register_index,
                })
            },
        })
    }
    /// 定数バッファパラメータ
    pub fn constant_buffer(
        visibility: D3D12_SHADER_VISIBILITY,
        register_index: u32,
        register_space: u32,
    ) -> Self {
        RootParameter(D3D12_ROOT_PARAMETER {
            ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
            ShaderVisibility: visibility,
            u: unsafe {
                *std::mem::transmute::<_, &_>(&D3D12_ROOT_DESCRIPTOR {
                    RegisterSpace: register_space,
                    ShaderRegister: register_index,
                })
            },
        })
    }
    /// Descriptor Tableから
    pub fn from_descriptor_table(
        visibility: D3D12_SHADER_VISIBILITY,
        descriptor_ranges: &[D3D12_DESCRIPTOR_RANGE],
    ) -> Self {
        RootParameter(D3D12_ROOT_PARAMETER {
            ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
            ShaderVisibility: visibility,
            u: unsafe {
                *std::mem::transmute::<_, &_>(&D3D12_ROOT_DESCRIPTOR_TABLE {
                    NumDescriptorRanges: descriptor_ranges.len() as _,
                    pDescriptorRanges: descriptor_ranges.as_ptr(),
                })
            },
        })
    }
}
/// 固定サンプラー
#[repr(transparent)]
pub struct StaticSampler(D3D12_STATIC_SAMPLER_DESC);
unsafe impl MarkForSameBits<D3D12_STATIC_SAMPLER_DESC> for StaticSampler {}
impl AsRef<D3D12_STATIC_SAMPLER_DESC> for StaticSampler {
    fn as_ref(&self) -> &D3D12_STATIC_SAMPLER_DESC {
        &self.0
    }
}
impl StaticSampler {
    /// 線形補間, 切り落とし
    pub fn linear_clamped(
        visibility: D3D12_SHADER_VISIBILITY,
        register_index: u32,
        register_space: u32,
    ) -> Self {
        StaticSampler(D3D12_STATIC_SAMPLER_DESC {
            Filter: D3D12_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            AddressV: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            AddressW: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D12_COMPARISON_FUNC_ALWAYS,
            BorderColor: D3D12_STATIC_BORDER_COLOR_TRANSPARENT_BLACK,
            MaxLOD: 0.0,
            MinLOD: 0.0,
            MipLODBias: D3D12_DEFAULT_MIP_LOD_BIAS,
            ShaderRegister: register_index,
            RegisterSpace: register_space,
            ShaderVisibility: visibility,
        })
    }
}
/// ルートシグネチャ
#[repr(transparent)]
pub struct RootSignature(*mut ID3D12RootSignature);
HandleWrapper!(for RootSignature[ID3D12RootSignature] + FromRawHandle);
impl Device {
    /// ルートシグネチャを作成
    pub fn new_root_signature(
        &self,
        params: &[RootParameter],
        samplers: &[StaticSampler],
        flags: D3D12_ROOT_SIGNATURE_FLAGS,
    ) -> IOResult<RootSignature> {
        let (p, ss) = (transmute_array(params), transmute_array(samplers));
        let desc = D3D12_ROOT_SIGNATURE_DESC {
            Flags: flags,
            pParameters: p.as_ptr(),
            NumParameters: p.len() as _,
            pStaticSamplers: ss.as_ptr(),
            NumStaticSamplers: ss.len() as _,
        };
        let (mut serialized, mut errmsg) = (std::ptr::null_mut(), std::ptr::null_mut());
        let hr = unsafe {
            D3D12SerializeRootSignature(
                &desc,
                D3D_ROOT_SIGNATURE_VERSION_1,
                &mut serialized,
                &mut errmsg,
            )
        };
        if !errmsg.is_null() {
            panic!("D3D12SerializeRootSignature Error: {:?}", unsafe {
                std::ffi::CStr::from_ptr((*errmsg).GetBufferPointer() as _)
            });
        }
        hr.checked().and_then(|_| unsafe {
            let mut handle = std::ptr::null_mut();
            (*self.0)
                .CreateRootSignature(
                    0,
                    (*serialized).GetBufferPointer(),
                    (*serialized).GetBufferSize(),
                    &ID3D12RootSignature::uuidof(),
                    &mut handle,
                )
                .to_result_with(|| RootSignature(handle as _))
        })
    }
}
impl RootSignature {
    /// シェーダバイナリから抽出
    pub fn from_shader_binary(device: &Device, bin: &[u8]) -> IOResult<Self> {
        let (mut rsb, mut sig) = (std::ptr::null_mut(), std::ptr::null_mut());
        unsafe {
            D3DGetBlobPart(
                bin.as_ptr() as *const _,
                bin.len() as _,
                D3D_BLOB_ROOT_SIGNATURE,
                0,
                &mut rsb,
            )
        }
        .checked()?;
        let rsb = ComPtr(rsb as *mut ID3DBlob);
        unsafe {
            (*device.0)
                .CreateRootSignature(
                    0,
                    (*rsb.0).GetBufferPointer(),
                    (*rsb.0).GetBufferSize(),
                    &ID3D12RootSignature::uuidof(),
                    &mut sig,
                )
                .to_result_with(|| RootSignature(sig as _))
        }
    }
}
unsafe impl Sync for RootSignature {}
unsafe impl Send for RootSignature {}

/// パイプラインステート
#[repr(transparent)]
pub struct PipelineState(*mut ID3D12PipelineState);
HandleWrapper!(for PipelineState[ID3D12PipelineState] + FromRawHandle);
impl Device {
    /// グラフィックス用パイプラインステートの作成
    pub fn new_graphics_pipeline_state(
        &self,
        desc: &D3D12_GRAPHICS_PIPELINE_STATE_DESC,
    ) -> IOResult<PipelineState> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.0).CreateGraphicsPipelineState(desc, &ID3D12PipelineState::uuidof(), &mut handle)
        }
        .to_result_with(|| PipelineState(handle as _))
    }
}
unsafe impl Sync for PipelineState {}
unsafe impl Send for PipelineState {}

/// パイプラインスナップショット
/// パイプラインステートの生成を手続き的にする
pub struct PipelineStateTracker<'d>(&'d Device, GraphicsPipelineStateDesc);
impl<'d> PipelineStateTracker<'d> {
    #![allow(dead_code)]

    /// 初期化
    pub fn new(factory: &'d Device) -> Self {
        PipelineStateTracker(
            factory,
            D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: std::ptr::null_mut(),
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                InputLayout: D3D12_INPUT_LAYOUT_DESC {
                    pInputElementDescs: std::ptr::null(),
                    NumElements: 0,
                },
                VS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: std::ptr::null(),
                    BytecodeLength: 0,
                },
                HS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: std::ptr::null(),
                    BytecodeLength: 0,
                },
                DS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: std::ptr::null(),
                    BytecodeLength: 0,
                },
                GS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: std::ptr::null(),
                    BytecodeLength: 0,
                },
                PS: D3D12_SHADER_BYTECODE {
                    pShaderBytecode: std::ptr::null(),
                    BytecodeLength: 0,
                },
                StreamOutput: D3D12_STREAM_OUTPUT_DESC {
                    pSODeclaration: std::ptr::null(),
                    NumEntries: 0,
                    pBufferStrides: std::ptr::null(),
                    NumStrides: 0,
                    RasterizedStream: 0,
                },
                BlendState: D3D12_BLEND_DESC {
                    AlphaToCoverageEnable: false as _,
                    IndependentBlendEnable: false as _,
                    RenderTarget: [*transmute_safe(&Blending::disabled()); 8],
                },
                SampleMask: D3D12_DEFAULT_SAMPLE_MASK,
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    FrontCounterClockwise: false as _,
                    DepthBias: D3D12_DEFAULT_DEPTH_BIAS as _,
                    DepthBiasClamp: D3D12_DEFAULT_DEPTH_BIAS_CLAMP,
                    SlopeScaledDepthBias: D3D12_DEFAULT_SLOPE_SCALED_DEPTH_BIAS,
                    DepthClipEnable: false as _,
                    MultisampleEnable: false as _,
                    AntialiasedLineEnable: false as _,
                    ForcedSampleCount: 0,
                    ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
                },
                DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
                    DepthEnable: false as _,
                    DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
                    DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
                    StencilEnable: false as _,
                    StencilReadMask: 0,
                    StencilWriteMask: 0,
                    FrontFace: D3D12_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D12_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D12_STENCIL_OP_KEEP,
                        StencilPassOp: D3D12_STENCIL_OP_KEEP,
                        StencilFunc: D3D12_COMPARISON_FUNC_ALWAYS,
                    },
                    BackFace: D3D12_DEPTH_STENCILOP_DESC {
                        StencilFailOp: D3D12_STENCIL_OP_KEEP,
                        StencilDepthFailOp: D3D12_STENCIL_OP_KEEP,
                        StencilPassOp: D3D12_STENCIL_OP_KEEP,
                        StencilFunc: D3D12_COMPARISON_FUNC_ALWAYS,
                    },
                },
                IBStripCutValue: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_0xFFFF,
                NumRenderTargets: 1,
                RTVFormats: [DXGI_FORMAT_UNKNOWN; 8],
                DSVFormat: DXGI_FORMAT_UNKNOWN,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                NodeMask: 0,
                CachedPSO: D3D12_CACHED_PIPELINE_STATE {
                    pCachedBlob: std::ptr::null(),
                    CachedBlobSizeInBytes: 0,
                },
                Flags: D3D12_PIPELINE_STATE_FLAG_NONE,
            },
        )
    }

    /// ルートシグネチャを設定
    pub fn set_root_signature(&mut self, root_signature: &RootSignature) -> &mut Self {
        self.1.pRootSignature = root_signature.0;
        self
    }
    /// プリミティブタイプの設定
    pub fn set_primitive_topology_type(
        &mut self,
        _type: D3D12_PRIMITIVE_TOPOLOGY_TYPE,
    ) -> &mut Self {
        self.1.PrimitiveTopologyType = _type;
        self
    }
    /// 頂点処理(シェーダと入力フォーマット)の設定
    pub fn set_vertex_processing<Shader: AsRef<D3D12_SHADER_BYTECODE>>(
        &mut self,
        shader: &Shader,
        input_format: &[D3D12_INPUT_ELEMENT_DESC],
    ) -> &mut Self {
        self.1.VS = *shader.as_ref();
        self.1.InputLayout = D3D12_INPUT_LAYOUT_DESC {
            pInputElementDescs: input_format.as_ptr(),
            NumElements: input_format.len() as _,
        };
        self
    }
    /// テッセレーション処理の設定
    pub fn set_tessellation_processing<
        HShader: AsRef<D3D12_SHADER_BYTECODE>,
        DShader: AsRef<D3D12_SHADER_BYTECODE>,
    >(
        &mut self,
        hull: &HShader,
        domain: &DShader,
    ) -> &mut Self {
        self.1.HS = *hull.as_ref();
        self.1.DS = *domain.as_ref();
        self
    }
    /// ジオメトリシェーダの設定
    pub fn set_geometry_shader<Shader: AsRef<D3D12_SHADER_BYTECODE>>(
        &mut self,
        shader: &Shader,
    ) -> &mut Self {
        self.1.GS = *shader.as_ref();
        self
    }
    /// ピクセルシェーダの設定
    pub fn set_pixel_shader<Shader: AsRef<D3D12_SHADER_BYTECODE>>(
        &mut self,
        shader: &Shader,
    ) -> &mut Self {
        self.1.PS = *shader.as_ref();
        self
    }
    /// 統一ブレンドステートの設定
    pub fn set_blend_state_common<RTBlend: AsRef<D3D12_RENDER_TARGET_BLEND_DESC>>(
        &mut self,
        state: &RTBlend,
        enable_alpha_to_coverage: bool,
    ) -> &mut Self {
        self.1.BlendState.AlphaToCoverageEnable = enable_alpha_to_coverage as _;
        self.1.BlendState.IndependentBlendEnable = false as _;
        self.1.BlendState.RenderTarget[0] = *state.as_ref();
        self
    }
    /// レンダーターゲットのフォーマット設定
    pub fn set_render_target_formats(&mut self, formats: &[DXGI_FORMAT]) -> &mut Self {
        self.1.NumRenderTargets = formats.len() as _;
        self.1.RTVFormats[..formats.len()].copy_from_slice(formats);
        for n in &mut self.1.RTVFormats[formats.len()..] {
            *n = DXGI_FORMAT_UNKNOWN;
        }
        self
    }

    /// 線のアンチエイリアスを設定
    pub fn set_line_antialiasing(&mut self, enabled: bool) -> &mut Self {
        self.1.RasterizerState.AntialiasedLineEnable = enabled as _;
        self
    }

    /// スナップショットをオブジェクト化
    pub fn make_state_object(&self) -> IOResult<PipelineState> {
        self.0.new_graphics_pipeline_state(&self.1)
    }
}
/// PipelineStateTrackerのメソッドチェーンサポート
#[macro_export]
macro_rules! StateBuilder
{
    (__ProcessInst($target: expr) => $snapshot: ident ; $($rest: tt)*) =>
    {
        let $snapshot = $target.make_state_object()?;
        StateBuilder!(__ProcessInst($target) $($rest)*);
    };
    (__ProcessInst($target: expr) $inst: ident $($args: expr),* ; $($rest: tt)*) =>
    {
        $target.$inst($($args),*);
        StateBuilder!(__ProcessInst($target) $($rest)*);
    };
    (__ProcessInst($target: expr)) => { /* term */ };
    ($object: expr => { $($lines: tt)* }) =>
    {
        let mut builder = $object;
        StateBuilder!(__ProcessInst(builder) $($lines)*);
    };
}

/// シェーダバイナリオブジェクト(Direct3D12ごかんのシェーダバイナリ構造をふくむ)
pub struct ShaderBinary(Vec<u8>, D3D12_SHADER_BYTECODE);
impl ShaderBinary {
    /// ファイルから読む
    pub fn from_file<ShaderPath: AsRef<Path> + ?Sized>(
        shader_path: &ShaderPath,
    ) -> std::io::Result<Self> {
        use std::io::prelude::*;

        std::fs::File::open(shader_path).and_then(|mut fp| {
            let mut data = Vec::new();
            fp.read_to_end(&mut data).map(|size| {
                let bc = D3D12_SHADER_BYTECODE {
                    pShaderBytecode: data.as_ptr() as _,
                    BytecodeLength: size as _,
                };
                ShaderBinary(data, bc)
            })
        })
    }
}
impl AsRef<D3D12_SHADER_BYTECODE> for ShaderBinary {
    fn as_ref(&self) -> &D3D12_SHADER_BYTECODE {
        &self.1
    }
}
impl Deref for ShaderBinary {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

/// ブレンディング
#[repr(transparent)]
pub struct Blending(D3D12_RENDER_TARGET_BLEND_DESC);
unsafe impl MarkForSameBits<D3D12_RENDER_TARGET_BLEND_DESC> for Blending {}
impl AsRef<D3D12_RENDER_TARGET_BLEND_DESC> for Blending {
    fn as_ref(&self) -> &D3D12_RENDER_TARGET_BLEND_DESC {
        &self.0
    }
}
impl Blending {
    /// 無効
    pub fn disabled() -> Self {
        Blending(D3D12_RENDER_TARGET_BLEND_DESC {
            BlendEnable: false as _,
            RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL as _,
            ..unsafe { std::mem::zeroed() }
        })
    }
    /// 乗算済みアルファ
    pub fn palpha() -> Self {
        Blending(D3D12_RENDER_TARGET_BLEND_DESC {
            BlendEnable: true as _,
            RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL as _,
            SrcBlend: D3D12_BLEND_ONE,
            DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
            BlendOp: D3D12_BLEND_OP_ADD,
            SrcBlendAlpha: D3D12_BLEND_ONE,
            DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
            BlendOpAlpha: D3D12_BLEND_OP_ADD,
            ..unsafe { std::mem::zeroed() }
        })
    }
}

pub use winapi::um::d3d12::{
    D3D12_FENCE_FLAG_NONE as FENCE_FLAG_NONE, D3D12_FENCE_FLAG_SHARED as FENCE_FLAG_SHARED,
    D3D12_FENCE_FLAG_SHARED_CROSS_ADAPTER as FENCE_FLAG_SHARED_CROSS_ADAPTER,
};

/// 同期オブジェクト(フェンス)
#[repr(transparent)]
pub struct Fence(*mut ID3D12Fence);
HandleWrapper!(for Fence[ID3D12Fence] + FromRawHandle);
impl Device {
    /// フェンスを作成
    pub fn new_fence(
        &self,
        initial_value: u64,
        flags: winapi::um::d3d12::D3D12_FENCE_FLAGS,
    ) -> IOResult<Fence> {
        let mut handle = std::ptr::null_mut();
        unsafe { (*self.0).CreateFence(initial_value, flags, &ID3D12Fence::uuidof(), &mut handle) }
            .to_result_with(|| Fence(handle as _))
    }
}
impl Fence {
    /// イベントの通知を設定する
    pub fn set_event_notification(&mut self, value: u64, event: HANDLE) -> IOResult<()> {
        unsafe { (*self.0).SetEventOnCompletion(value, event) }.checked()
    }

    /// 現在の値を取得
    pub fn completed_value(&self) -> u64 {
        unsafe { (*self.0).GetCompletedValue() }
    }

    /// 現在の値を設定
    pub fn signal(&mut self, new_value: u64) -> IOResult<()> {
        unsafe { (*self.0).Signal(new_value) }.checked()
    }
}
unsafe impl AsRawHandle<ID3D12DeviceChild> for Fence {
    fn as_raw_handle(&self) -> *mut ID3D12DeviceChild {
        self.0 as _
    }
}
unsafe impl Sync for Fence {}
unsafe impl Send for Fence {}

/// グラフィックス操作用のコマンドリスト
#[repr(transparent)]
pub struct GraphicsCommandList(*mut ID3D12GraphicsCommandList);
HandleWrapper!(for GraphicsCommandList[ID3D12GraphicsCommandList] + FromRawHandle);
impl Device {
    /// グラフィックス操作用のコマンドリストを作る(初期状態では記録するようになってる)
    pub fn new_graphics_command_list(
        &self,
        alloc: &mut CommandAllocator,
        initial_ps: Option<&PipelineState>,
    ) -> IOResult<GraphicsCommandList> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            (*self.0).CreateCommandList(
                0,
                alloc.1 as _,
                alloc.0,
                initial_ps.map(|x| x.0).unwrap_or(std::ptr::null_mut()),
                &ID3D12GraphicsCommandList::uuidof(),
                &mut handle,
            )
        }
        .to_result_with(|| GraphicsCommandList(handle as _))
    }
}
impl GraphicsCommandList {
    /// 記録おしまい
    pub fn close(&mut self) -> IOResult<()> {
        unsafe { (*self.0).Close().checked() }
    }
    /// コマンドリストの初期化
    pub fn reset(
        &mut self,
        alloc: &CommandAllocator,
        initial_ps: Option<&PipelineState>,
    ) -> IOResult<&mut Self> {
        unsafe {
            (*self.0)
                .Reset(
                    alloc.0,
                    initial_ps.map(|x| x.0).unwrap_or(std::ptr::null_mut()),
                )
                .to_result(self)
        }
    }
    /// リソースバリアを張る
    pub fn resource_barrier(&mut self, barriers: &[ResourceBarrier]) -> &mut Self {
        let b = transmute_array(barriers);
        unsafe { (*self.0).ResourceBarrier(b.len() as _, b.as_ptr()) };
        self
    }

    /// リソースをコピーする
    pub fn copy_resource(&mut self, src: &Resource, dst: &Resource) -> &mut Self {
        unsafe { (*self.0).CopyResource(dst.0, src.0) };
        self
    }
    /// バッファの一部をコピーする
    pub fn copy_buffer_region(
        &mut self,
        src: &Resource,
        range: std::ops::Range<usize>,
        dst: &Resource,
        dst_offset: usize,
    ) -> &mut Self {
        unsafe {
            (*self.0).CopyBufferRegion(
                dst.0,
                dst_offset as _,
                src.0,
                range.start as _,
                (range.end - range.start) as _,
            )
        };
        self
    }
    /// テクスチャの一部をコピーする
    pub fn copy_texture_region(
        &mut self,
        src: &D3D12_TEXTURE_COPY_LOCATION,
        src_box: Option<&D3D12_BOX>,
        dst: &D3D12_TEXTURE_COPY_LOCATION,
        dst_x: u32,
        dst_y: u32,
        dst_z: u32,
    ) -> &mut Self {
        unsafe {
            (*self.0).CopyTextureRegion(
                dst,
                dst_x,
                dst_y,
                dst_z,
                src,
                src_box.map_or(std::ptr::null(), |p| p as _),
            );
        }
        self
    }
    /// レンダーターゲットをクリア
    pub fn clear_render_target_view(
        &mut self,
        target: D3D12_CPU_DESCRIPTOR_HANDLE,
        color: &[f32; 4],
    ) -> &mut Self {
        unsafe { (*self.0).ClearRenderTargetView(target, color, 0, std::ptr::null()) };
        self
    }

    /// パイプラインステート/ルートシグネチャの設定
    pub fn set_pipeline_state(
        &mut self,
        ps: &PipelineState,
        signature: Option<&RootSignature>,
    ) -> &mut Self {
        unsafe { (*self.0).SetPipelineState(ps.0) };
        if let Some(sig) = signature {
            self.set_root_signature(sig)
        } else {
            self
        }
    }
    /// ルートシグネチャのみ設定
    pub fn set_root_signature(&mut self, signature: &RootSignature) -> &mut Self {
        unsafe { (*self.0).SetGraphicsRootSignature(signature.0) };
        self
    }
    /// ルート定数の設定(複数)
    pub fn set_root_constants(
        &mut self,
        param_index: u32,
        offset: u32,
        values: &[f32],
    ) -> &mut Self {
        unsafe {
            (*self.0).SetGraphicsRoot32BitConstants(
                param_index,
                values.len() as _,
                values.as_ptr() as _,
                offset,
            )
        };
        self
    }
    /// ルート定数の設定(ひとつ)
    pub fn set_root_constant<C: RootConstant>(
        &mut self,
        param_index: u32,
        offset: u32,
        value: C,
    ) -> &mut Self {
        unsafe {
            (*self.0).SetGraphicsRoot32BitConstant(param_index, value.passing_form(), offset)
        };
        self
    }
    /// ルート定数バッファの設定
    pub fn set_root_constant_buffer(
        &mut self,
        param_index: u32,
        resource_ptr: D3D12_GPU_VIRTUAL_ADDRESS,
    ) -> &mut Self {
        unsafe { (*self.0).SetGraphicsRootConstantBufferView(param_index, resource_ptr) };
        self
    }
    /// ルートリソースバッファの設定
    pub fn set_root_resource_buffer(
        &mut self,
        param_index: u32,
        resource_ptr: D3D12_GPU_VIRTUAL_ADDRESS,
    ) -> &mut Self {
        unsafe {
            (*self.0).SetGraphicsRootShaderResourceView(param_index, resource_ptr);
        }
        self
    }
    /// 参照されるデスクリプタヒープの設定
    pub fn set_descriptor_heaps(&mut self, heaps: &[*mut ID3D12DescriptorHeap]) -> &mut Self {
        unsafe { (*self.0).SetDescriptorHeaps(heaps.len() as _, heaps.as_ptr() as *mut _) };
        self
    }
    /// デスクリプタテーブルを設定
    pub fn set_root_descriptor_table(
        &mut self,
        param_index: u32,
        table_start: &DeviceDescriptorHandle,
    ) -> &mut Self {
        unsafe { (*self.0).SetGraphicsRootDescriptorTable(param_index, table_start.0) };
        self
    }
    /// レンダーターゲットの設定
    pub fn set_render_targets(
        &mut self,
        handles: &[D3D12_CPU_DESCRIPTOR_HANDLE],
        depth_stencil: Option<D3D12_CPU_DESCRIPTOR_HANDLE>,
    ) -> &mut Self {
        unsafe {
            (*self.0).OMSetRenderTargets(
                handles.len() as _,
                handles.as_ptr(),
                false as _,
                depth_stencil
                    .as_ref()
                    .map(|x| x as _)
                    .unwrap_or(std::ptr::null()),
            )
        };
        self
    }
    /// ビューポートと切りぬきエリアの設定
    pub fn set_view_states(&mut self, vps: &[(D3D12_VIEWPORT, D3D12_RECT)]) -> &mut Self {
        let (vps, scis): (Vec<_>, Vec<_>) = vps.iter().cloned().unzip();
        self.set_viewports(&vps).set_scissor_rects(&scis)
    }
    /// ビューポートだけ更新
    pub fn set_viewports(&mut self, vps: &[D3D12_VIEWPORT]) -> &mut Self {
        unsafe { (*self.0).RSSetViewports(vps.len() as _, vps.as_ptr()) };
        self
    }
    /// 切りぬきエリアだけ更新
    pub fn set_scissor_rects(&mut self, scis: &[D3D12_RECT]) -> &mut Self {
        unsafe { (*self.0).RSSetScissorRects(scis.len() as _, scis.as_ptr()) };
        self
    }

    /// プリミティブトポロジを指定
    pub fn set_primitive_topology(&mut self, tp: D3D12_PRIMITIVE_TOPOLOGY) -> &mut Self {
        unsafe { (*self.0).IASetPrimitiveTopology(tp) };
        self
    }
    /// 頂点バッファの設定
    pub fn set_vertex_buffers(
        &mut self,
        slot_from: u32,
        buffers: &[VertexBufferView],
    ) -> &mut Self {
        unsafe { (*self.0).IASetVertexBuffers(slot_from, buffers.len() as _, buffers.as_ptr()) };
        self
    }
    /// インデックスバッファの設定
    pub fn set_index_buffer(&mut self, buffer: &D3D12_INDEX_BUFFER_VIEW) -> &mut Self {
        unsafe { (*self.0).IASetIndexBuffer(buffer) };
        self
    }

    /// ドローコールを発行
    pub fn draw(&mut self, vertex_count: u32, instance_count: u32) -> &mut Self {
        unsafe { (*self.0).DrawInstanced(vertex_count, instance_count, 0, 0) };
        self
    }
    /// インデックスを使うドローコールを発行
    pub fn draw_indexed(
        &mut self,
        index_count: u32,
        instance_count: u32,
        vertex_offset: i32,
    ) -> &mut Self {
        unsafe { (*self.0).DrawIndexedInstanced(index_count, instance_count, 0, vertex_offset, 0) };
        self
    }

    /// バンドルバッファを実行
    pub fn execute(&mut self, cmd: &GraphicsCommandList) -> &mut Self {
        unsafe { (*self.0).ExecuteBundle(cmd.0) };
        self
    }
    /// コマンドインジェクション(チェーン中にifとかで分かれたい場合)
    pub fn inject(&mut self, injector: impl FnOnce(&mut Self) -> &mut Self) -> &mut Self {
        injector(self)
    }
}
/// GraphicsCommandListのメソッドチェーンサポート
#[macro_export]
macro_rules! GraphicsCommandAssembly
{
    (__ProcessInst($target: expr) root_constants[$index: expr, $offset: expr] = &[$($values: expr),*]; $($rest: tt)*) =>
    {
        // Translated: set_root_constants
        GraphicsCommandAssembly!(__ProcessInst($target) set_root_constants $index, $offset, &[$($values),*]; $($rest)*);
    };
    (__ProcessInst($target: expr) root_constants[$index: expr, $offset: expr] = $value: expr; $($rest: tt)*) =>
    {
        // Translated: set_root_constant
        GraphicsCommandAssembly!(__ProcessInst($target) set_root_constant $index, $offset, $value; $($rest)*);
    };
    (__ProcessInst($target: expr) root_constants[$index: expr] = &[$($values: expr),*]; $($rest: tt)*) =>
    {
        // Translated: set_root_constants
        GraphicsCommandAssembly!(__ProcessInst($target) set_root_constants $index, 0, &[$($values),*]; $($rest)*);
    };
    (__ProcessInst($target: expr) root_constants[$index: expr] = $value: expr; $($rest: tt)*) =>
    {
        // Translated: set_root_constant
        GraphicsCommandAssembly!(__ProcessInst($target) set_root_constant $index, 0, $value; $($rest)*);
    };
    (__ProcessInst($target: expr) !i[$voffset: expr] $vertices: expr, $instances: expr; $($rest: tt)*) =>
    {
        // Translated: draw_indexed
        GraphicsCommandAssembly!(__ProcessInst($target) draw_indexed $vertices as _, $instances as _, $voffset as _; $($rest)*);
    };
    (__ProcessInst($target: expr) !i[$voffset: expr] $vertices: expr; $($rest: tt)*) =>
    {
        // Translated: draw_indexed
        GraphicsCommandAssembly!(__ProcessInst($target) draw_indexed $vertices as _, 1, $voffset as _; $($rest)*);
    };
    (__ProcessInst($target: expr) !i $vertices: expr, $instances: expr; $($rest: tt)*) =>
    {
        // Translated: draw_indexed
        GraphicsCommandAssembly!(__ProcessInst($target) draw_indexed $vertices as _, $instances as _, 0; $($rest)*);
    };
    (__ProcessInst($target: expr) !i $vertices: expr; $($rest: tt)*) =>
    {
        // Translated: draw_indexed
        GraphicsCommandAssembly!(__ProcessInst($target) draw_indexed $vertices as _, 1, 0; $($rest)*);
    };
    (__ProcessInst($target: expr) ~ $($barriers: expr),*; $($rest: tt)*) =>
    {
        // Translated: resource_barrier
        GraphicsCommandAssembly!(__ProcessInst($target) resource_barrier &[$($barriers),*]; $($rest)*);
    };
    (__ProcessInst($target: expr) > $cmdbuf: expr; $($rest: tt)*) =>
    {
        // Translated: execute
        GraphicsCommandAssembly!(__ProcessInst($target) execute $cmdbuf; $($rest)*);
    };
    (__ProcessInst($target: expr) ! $vertices: expr, $instances: expr; $($rest: tt)*) =>
    {
        // Translated: draw
        GraphicsCommandAssembly!(__ProcessInst($target) draw $vertices as _, $instances as _; $($rest)*);
    };
    (__ProcessInst($target: expr) ! $vertices: expr; $($rest: tt)*) =>
    {
        // Translated: draw
        GraphicsCommandAssembly!(__ProcessInst($target) draw $vertices as _, 1; $($rest)*);
    };
    (__ProcessInst($target: expr) $inst: ident $($args: expr),* ; $($rest: tt)*) =>
    {
        $target.$inst($($args),*);
        GraphicsCommandAssembly!(__ProcessInst($target) $($rest)*);
    };
    (__ProcessInst($target: expr)) => { /* term */ };

    ($object: expr => { $($lines: tt)* }) =>
    {
        {
            let asm_builder = $object;
            GraphicsCommandAssembly!(__ProcessInst(asm_builder) $($lines)*);
            asm_builder.close()
        }
    };
}
unsafe impl Sync for GraphicsCommandList {}
unsafe impl Send for GraphicsCommandList {}

/// ルート定数として使用できるやつ(32bit限定)
pub trait RootConstant {
    fn passing_form(self) -> u32;
}
impl RootConstant for f32 {
    fn passing_form(self) -> u32 {
        unsafe { std::mem::transmute(self) }
    }
}
impl RootConstant for i32 {
    fn passing_form(self) -> u32 {
        unsafe { std::mem::transmute(self) }
    }
}
impl RootConstant for u32 {
    fn passing_form(self) -> u32 {
        self
    }
}

/// リソースバリア
#[repr(transparent)]
pub struct ResourceBarrier(D3D12_RESOURCE_BARRIER);
unsafe impl MarkForSameBits<D3D12_RESOURCE_BARRIER> for ResourceBarrier {}
impl AsRef<D3D12_RESOURCE_BARRIER> for ResourceBarrier {
    fn as_ref(&self) -> &D3D12_RESOURCE_BARRIER {
        &self.0
    }
}
impl ResourceBarrier {
    /// エイリアシング(リソースの有効化)
    pub fn aliasing(before: Option<&Resource>, after: &Resource) -> Self {
        ResourceBarrier(D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_ALIASING,
            Flags: 0,
            u: unsafe {
                *std::mem::transmute::<_, &_>(&D3D12_RESOURCE_ALIASING_BARRIER {
                    pResourceBefore: before.map(|x| x.0).unwrap_or(std::ptr::null_mut()),
                    pResourceAfter: after.0,
                })
            },
        })
    }
    /// トランジション(リソースの状態を変える)
    pub fn transition(
        target: &Resource,
        before: D3D12_RESOURCE_STATES,
        after: D3D12_RESOURCE_STATES,
    ) -> Self {
        ResourceBarrier(D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            Flags: 0,
            u: unsafe {
                *std::mem::transmute::<_, &_>(&D3D12_RESOURCE_TRANSITION_BARRIER {
                    pResource: target.0,
                    Subresource: 0,
                    StateBefore: before,
                    StateAfter: after,
                })
            },
        })
    }
}

/// 頂点バッファビューの作成
pub fn vertex_buffer_view<T>(
    location: GraphicsVirtualPtr,
    element_count: usize,
) -> VertexBufferView {
    VertexBufferView {
        BufferLocation: location.0,
        StrideInBytes: std::mem::size_of::<T>() as _,
        SizeInBytes: (std::mem::size_of::<T>() * element_count) as _,
    }
}
/// インデックスバッファビューの作成
pub fn index_buffer_view(location: GraphicsVirtualPtr, element_count: usize) -> IndexBufferView {
    IndexBufferView {
        BufferLocation: location.0,
        SizeInBytes: (size_of::<u16>() * element_count) as _,
        Format: DXGI_FORMAT_R16_UINT,
    }
}
/// GPU仮想アドレスのラップ
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct GraphicsVirtualPtr(pub D3D12_GPU_VIRTUAL_ADDRESS);
impl GraphicsVirtualPtr {
    pub fn offset(self, offs: isize) -> Self {
        GraphicsVirtualPtr(self.0 + offs as D3D12_GPU_VIRTUAL_ADDRESS)
    }
}

/// ビューポート補助
#[repr(C)]
pub struct Viewport {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
    pub min_depth: f32,
    pub max_depth: f32,
}
unsafe impl MarkForSameBits<D3D12_VIEWPORT> for Viewport {}
impl AsRef<D3D12_VIEWPORT> for Viewport {
    fn as_ref(&self) -> &D3D12_VIEWPORT {
        unsafe { std::mem::transmute(self) }
    }
}
impl Default for Viewport {
    fn default() -> Self {
        Viewport {
            left: 0.0,
            top: 0.0,
            width: 256.0,
            height: 256.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}
impl Viewport {
    /// 右はしをセット
    pub fn set_right(&mut self, r: f32) {
        self.width = r - self.left;
    }
    /// 下はしをセット
    pub fn set_bottom(&mut self, b: f32) {
        self.height = b - self.top;
    }

    /// 左上から収縮
    pub fn shrink_lt(&self, amount: f32) -> Self {
        Viewport {
            left: self.left + amount,
            top: self.top + amount,
            width: self.width - amount,
            height: self.height - amount,
            ..*self
        }
    }
}

/// D3D12_TEXTURE_COPY_LOCATION constructor補助
#[repr(transparent)]
pub struct TextureCopyLocation(D3D12_TEXTURE_COPY_LOCATION);
impl TextureCopyLocation {
    pub fn with_placed_footprint<R: AsRawHandle<ID3D12Resource> + ?Sized>(
        resource: &R,
        footprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
    ) -> Self {
        let mut u = std::mem::MaybeUninit::<D3D12_TEXTURE_COPY_LOCATION_u>::uninit();
        unsafe {
            *(*u.as_mut_ptr()).PlacedFootprint_mut() = footprint;
        }
        TextureCopyLocation(D3D12_TEXTURE_COPY_LOCATION {
            pResource: resource.as_raw_handle(),
            Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
            u: unsafe { u.assume_init() },
        })
    }
    pub fn with_subresource_index<R: AsRawHandle<ID3D12Resource> + ?Sized>(
        resource: &R,
        subresource_index: u32,
    ) -> Self {
        let mut u = std::mem::MaybeUninit::<D3D12_TEXTURE_COPY_LOCATION_u>::uninit();
        unsafe {
            *(*u.as_mut_ptr()).SubresourceIndex_mut() = subresource_index;
        }
        TextureCopyLocation(D3D12_TEXTURE_COPY_LOCATION {
            pResource: resource.as_raw_handle(),
            Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
            u: unsafe { u.assume_init() },
        })
    }
}
impl From<TextureCopyLocation> for D3D12_TEXTURE_COPY_LOCATION {
    fn from(v: TextureCopyLocation) -> Self {
        v.0
    }
}
impl std::ops::Deref for TextureCopyLocation {
    type Target = D3D12_TEXTURE_COPY_LOCATION;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait DeviceFeature {
    const FEATURE_TYPE: D3D12_FEATURE;
}
impl Device {
    pub fn check_feature_support<F: DeviceFeature>(&self, feature: &mut F) -> IOResult<()> {
        unsafe {
            (*self.0).CheckFeatureSupport(
                F::FEATURE_TYPE,
                feature as *mut _ as _,
                std::mem::size_of::<F>() as _,
            )
        }
        .checked()
    }
}
impl DeviceFeature for D3D12_FEATURE_DATA_MULTISAMPLE_QUALITY_LEVELS {
    const FEATURE_TYPE: D3D12_FEATURE = D3D12_FEATURE_MULTISAMPLE_QUALITY_LEVELS;
}
impl DeviceFeature for D3D12_FEATURE_DATA_D3D12_OPTIONS {
    const FEATURE_TYPE: D3D12_FEATURE = D3D12_FEATURE_D3D12_OPTIONS;
}

#[link(name = "d3d12")]
extern "system" {
    fn D3D12CreateDevice(
        pAdapter: *mut IUnknown,
        MinimumFeatureLevel: D3D_FEATURE_LEVEL,
        riid: REFIID,
        ppDevice: *mut *mut c_void,
    ) -> HRESULT;
    fn D3D12GetDebugInterface(riid: REFIID, ppvDebug: *mut *mut c_void) -> HRESULT;
    fn D3D12SerializeRootSignature(
        pRootSignature: *const D3D12_ROOT_SIGNATURE_DESC,
        Version: D3D_ROOT_SIGNATURE_VERSION,
        ppBlob: *mut *mut ID3DBlob,
        ppErrorBlob: *mut *mut ID3DBlob,
    ) -> HRESULT;
}
