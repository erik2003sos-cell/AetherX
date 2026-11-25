// VIctory MotionFlow: vendor-neutral D3D12 compute kernels.
// Initial prototype: block-matching optical flow + 2x midpoint synthesis.
// No vendor SDK, AI model, depth buffer, or motion-vector input is required.
// The host must bind linear/single-sample color textures and dispatch these
// kernels in order. Flow is stored on a configurable coarse grid in pixel units.

Texture2D<float4> PreviousFrame : register(t0);
Texture2D<float4> CurrentFrame  : register(t1);
Texture2D<float2> FlowField     : register(t2);
SamplerState LinearClamp       : register(s0);

// Both entry points share one explicit D3D12 root signature. Ten root
// constants match MotionFlowParams exactly; the three SRVs and two UAVs
// are contiguous descriptor tables, with a separate linear-clamp sampler.
#define MOTIONFLOW_ROOT_SIGNATURE \
    "RootConstants(num32BitConstants=10, b0), " \
    "DescriptorTable(SRV(t0, numDescriptors=3)), " \
    "DescriptorTable(Sampler(s0)), " \
    "DescriptorTable(UAV(u0, numDescriptors=2))"
RWTexture2D<float2> OutFlow     : register(u0);
RWTexture2D<float4> OutFrame    : register(u1);

cbuffer MotionFlowParams : register(b0)
{
    uint2 ImageSize;
    uint2 FlowSize;
    uint SearchRadius;
    uint PatchRadius;
    uint FlowScale;
    float BlendFactor;
    float ExposureScale;
    uint Padding;
};


float Luma(float3 c)
{
    return dot(c, float3(0.2126, 0.7152, 0.0722));
}

bool Inside(int2 p)
{
    return all(p >= 0) && all(p < int2(ImageSize));
}

// Estimate the displacement from PreviousFrame to CurrentFrame at each
// output pixel using a small local SAD search. This is deliberately a
// conservative first implementation, not a claim of production quality.
[RootSignature(MOTIONFLOW_ROOT_SIGNATURE)]
[numthreads(8, 8, 1)]
void EstimateFlow(uint3 tid : SV_DispatchThreadID)
{
    int2 flowPixel = int2(tid.xy);
    if (any(flowPixel >= int2(FlowSize))) return;

    // Estimate on a quarter-resolution grid to keep the search practical.
    int2 p = min(flowPixel * (int)FlowScale + int(FlowScale / 2), int2(ImageSize) - 1);
    float bestCost = 1.0e20;
    int2 bestMotion = int2(0, 0);

    [loop]
    for (int y = -(int)SearchRadius; y <= (int)SearchRadius; ++y)
    {
        [loop]
        for (int x = -(int)SearchRadius; x <= (int)SearchRadius; ++x)
        {
            int2 motion = int2(x, y) * (int)FlowScale;
            float cost = 0.0;
            uint samples = 0;

            [loop]
            for (int py = -(int)PatchRadius; py <= (int)PatchRadius; ++py)
            {
                [loop]
                for (int px = -(int)PatchRadius; px <= (int)PatchRadius; ++px)
                {
                    int2 q = p + int2(px, py) * (int)FlowScale;
                    int2 oldQ = q - motion;
                    if (!Inside(q) || !Inside(oldQ)) continue;

                    float a = Luma(CurrentFrame.Load(int3(q, 0)).rgb);
                    float b = Luma(PreviousFrame.Load(int3(oldQ, 0)).rgb);
                    cost += abs(a - b);
                    ++samples;
                }
            }

            if (samples > 0)
            {
                cost /= samples;
                if (cost < bestCost)
                {
                    bestCost = cost;
                    bestMotion = motion;
                }
            }
        }
    }

    // Reject ambiguous/flat matches rather than inventing large motion.
    float confidence = saturate(1.0 - bestCost * 8.0);
    OutFlow[flowPixel] = float2(bestMotion) * confidence;
}

// Synthesize the temporal midpoint. Flow points from the previous frame
// toward the current frame. A future host integration must handle HUD masks,
// camera cuts, frame pacing, resource barriers, and swapchain lifetime.
[RootSignature(MOTIONFLOW_ROOT_SIGNATURE)]
[numthreads(8, 8, 1)]
void SynthesizeMidpoint(uint3 tid : SV_DispatchThreadID)
{
    int2 p = int2(tid.xy);
    if (!Inside(p)) return;

    int2 flowPixel = min(p / (int)FlowScale, int2(FlowSize) - 1);
    float2 motion = FlowField.Load(int3(flowPixel, 0));
    float2 halfMotion = motion * 0.5;

    float2 nowUv = (float2(p) + 0.5 - halfMotion) / float2(ImageSize);
    float2 oldUv = (float2(p) + 0.5 + halfMotion) / float2(ImageSize);

    float4 nowColor = CurrentFrame.SampleLevel(LinearClamp, nowUv, 0);
    float4 oldColor = PreviousFrame.SampleLevel(LinearClamp, oldUv, 0);
    OutFrame[p] = lerp(oldColor, nowColor, saturate(BlendFactor)) * float4(ExposureScale, ExposureScale, ExposureScale, 1.0);
}
