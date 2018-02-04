//! Direct3D Common Exports

use winapi::um::d3dcommon::*;
use d3d12; use std::io::Result as IOResult;
use std::borrow::Cow;

#[repr(u32)] #[derive(Clone, Copy)] #[allow(non_camel_case_types, dead_code)]
pub enum FeatureLevel
{
    v11 = D3D_FEATURE_LEVEL_11_0,
    v11_1 = D3D_FEATURE_LEVEL_11_1,
    v12 = D3D_FEATURE_LEVEL_12_0
}

/// シェーダ生成のもとになるオブジェクト
pub trait ShaderSource { fn binary(&self) -> IOResult<Cow<[u8]>>; }
/// ファイルパス
impl ShaderSource for str
{
    fn binary(&self) -> IOResult<Cow<[u8]>>
    {
        use std::io::Read; use std::fs::File;
        File::open(self).and_then(|mut fp| { let mut b = Vec::new(); fp.read_to_end(&mut b).map(|_| b.into()) })
    }
}
/// ロード済みバイナリ
impl ShaderSource for d3d12::ShaderBinary
{
    fn binary(&self) -> IOResult<Cow<[u8]>> { Ok(Cow::Borrowed(self)) }
}
