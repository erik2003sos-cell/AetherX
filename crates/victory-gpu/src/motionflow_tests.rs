//! Windows-only validation that both MotionFlow entry points carry a
//! valid embedded D3D12 root signature and can create compute PSOs.
use windows::Win32::Graphics::Direct3D::Fxc::{
    D3D_BLOB_ROOT_SIGNATURE, D3DCompile, D3DGetBlobPart, ID3DBlob,
};
use windows::Win32::Graphics::Direct3D12::D3D12_COMPUTE_PIPELINE_STATE_DESC;
use windows::core::s;

#[test]
fn both_motionflow_kernels_compile_and_create_compute_pipelines() {
    // Shader compilation and root-signature extraction do not require a GPU.
    // PSO creation is additionally exercised whenever the host D3D12 device
    // is available on this machine.
    let host = crate::host::host();
    let source = include_str!("../shaders/motionflow.hlsl");
    for entry in ["EstimateFlow", "SynthesizeMidpoint"] {
        let mut code: Option<ID3DBlob> = None;
        let mut errors: Option<ID3DBlob> = None;
        let entry_name = std::ffi::CString::new(entry).unwrap();
        // SAFETY: source and entry point remain alive for the duration of
        // compilation; output blobs are owned by the caller.
        unsafe {
            D3DCompile(
                source.as_ptr().cast(),
                source.len(),
                s!("motionflow.hlsl"),
                None,
                None,
                windows::core::PCSTR(entry_name.as_ptr().cast()),
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
                panic!("{entry_name:?} failed to compile: {error}: {diagnostic}");
            });
        }
        let code = code.expect("D3DCompile returned a shader blob");
        // D3D12 CreateRootSignature consumes a serialized root-signature
        // blob, not the complete shader bytecode. Extract the embedded
        // D3D_BLOB_ROOT_SIGNATURE part before passing it to D3D12.
        // SAFETY: code owns the byte range for the duration of extraction.
        let root_blob = unsafe {
            let bytes = std::slice::from_raw_parts(
                code.GetBufferPointer().cast::<u8>(),
                code.GetBufferSize(),
            );
            D3DGetBlobPart(
                bytes.as_ptr().cast(),
                bytes.len(),
                D3D_BLOB_ROOT_SIGNATURE,
                0,
            )
        }
        .unwrap_or_else(|error| panic!("{entry_name:?} has no valid embedded root signature: {error}"));

        if let Some(host) = host {
            // SAFETY: both blobs remain alive through root-signature and PSO
            // creation; the root signature bytes came from D3DGetBlobPart.
            unsafe {
                let root_bytes = std::slice::from_raw_parts(
                    root_blob.GetBufferPointer().cast::<u8>(),
                    root_blob.GetBufferSize(),
                );
                let root = host
                    .device
                    .CreateRootSignature(0, root_bytes)
                    .expect("MotionFlow embedded root signature");
                let shader_bytes = std::slice::from_raw_parts(
                    code.GetBufferPointer().cast::<u8>(),
                    code.GetBufferSize(),
                );
                let desc = D3D12_COMPUTE_PIPELINE_STATE_DESC {
                    pRootSignature: std::mem::transmute_copy(&root),
                    CS: windows::Win32::Graphics::Direct3D12::D3D12_SHADER_BYTECODE {
                        pShaderBytecode: shader_bytes.as_ptr().cast(),
                        BytecodeLength: shader_bytes.len(),
                    },
                    ..Default::default()
                };
                host.device
                    .CreateComputePipelineState(&desc)
                    .expect("MotionFlow compute pipeline");
            }
        }
    }
}
