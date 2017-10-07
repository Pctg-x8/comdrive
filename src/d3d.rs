//! Direct3D Common Exports

use winapi::um::d3dcommon::*;

#[repr(u32)] #[derive(Clone, Copy)] #[allow(non_camel_case_types, dead_code)]
pub enum FeatureLevel
{
    v11 = D3D_FEATURE_LEVEL_11_0,
    v11_1 = D3D_FEATURE_LEVEL_11_1,
    v12 = D3D_FEATURE_LEVEL_12_0
}

