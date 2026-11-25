//! Windows-only validation that both MotionFlow entry points carry a
//! valid embedded D3D12 root signature and can create compute PSOs.
use windows::Win32::Graphics::Direct3D::Fxc::{D3DCompile, ID3DBlob};
use windows::Win32::Graphics::Direct3D12::D3D12_COMPUTE_PIPELINE_STATE_DESC;
use windows::core::s;

#[test]
fn both_motionflow_kernels_compile_and_create_compute_pipelines() {
    let Some(host) = crate::host::host() else {
        return;
    };
    let source = include_str!("../shaders/motionflow.hlsl");
    for entry in ["EstimateFlow", "SynthesizeMidpoint"] {
        let mut code: Option<ID3DBlob> = None;
        let mut errors: Option<ID3DBlob> = None;
        let entry = std::ffi::CString::new(entry).unwrap();
        // SAFETY: source and entry point remain alive for the duration of
        // compilation; output blobs are owned by the caller.
        unsafe {
            D3DCompile(
                source.as_ptr().cast(),
                source.len(),
                s!("motionflow.hlsl"),
                None,
                None,
                windows::core::PCSTR(entry.as_ptr().cast()),
                s!("cs_5_1"),
                0,
                0,
                &mut code,
                Some(&mut errors),
            )
            .unwrap_or_else(|error| {
                let diagnostic = errors.as_ref().map_or_else(
                    || String::from("no compiler diagnostic"),
                    |blob| {
                        // SAFETY: the compiler error blob owns this byte range.
                        let bytes = std::slice::from_raw_parts(
                            blob.GetBufferPointer().cast::<u8>(),
                            blob.GetBufferSize(),
                        );
                        String::from_utf8_lossy(bytes).into_owned()
                    },
                );
                panic!("{entry:?} failed to compile: {error}: {diagnostic}");
            });
        }
        let code = code.expect("D3DCompile returned a shader blob");
        // SAFETY: bytecode remains alive through root-signature and PSO creation.
        unsafe {
            let bytes = std::slice::from_raw_parts(
                code.GetBufferPointer().cast::<u8>(),
                code.GetBufferSize(),
            );
            let root = host
                .device
                .CreateRootSignature(0, bytes)
                .expect("MotionFlow embedded root signature");
            let desc = D3D12_COMPUTE_PIPELINE_STATE_DESC {
                pRootSignature: std::mem::transmute_copy(&root),
                CS: windows::Win32::Graphics::Direct3D12::D3D12_SHADER_BYTECODE {
                    pShaderBytecode: bytes.as_ptr().cast(),
                    BytecodeLength: bytes.len(),
                },
                ..Default::default()
            };
            host.device
                .CreateComputePipelineState(&desc)
                .expect("MotionFlow compute pipeline");
        }
    }
}
