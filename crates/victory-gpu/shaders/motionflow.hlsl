// VIctory MotionFlow: vendor-neutral D3D12 compute kernels.
// Initial prototype: block-matching optical flow + 2x midpoint synthesis.
// No vendor SDK, AI model, depth buffer, or motion-vector input is required.
// The host must bind linear/single-sample color textures and dispatch these
// kernels in order. Flow is stored at full resolution in pixel units.

Texture2D<float4> PreviousFrame : register(t0);
Texture2D<float4> CurrentFrame  : register(t1);
Texture2D<float2> FlowField     : register(t2);
RWTexture2D<float2> OutFlow     : register(u0);
RWTexture2D<float4> OutFrame    : register(u1);

cbuffer MotionFlowParams : register(b0)
{
    uint2 ImageSize;
    uint SearchRadius;
    uint PatchRadius;
    float BlendFactor;
    float ExposureScale;
    uint2 Padding;
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
[numthreads(8, 8, 1)]
void EstimateFlow(uint3 tid : SV_DispatchThreadID)
{
    int2 p = int2(tid.xy);
    if (!Inside(p)) return;

    float bestCost = 1.0e20;
    int2 bestMotion = int2(0, 0);
    float centerNow = Luma(CurrentFrame.Load(int3(p, 0)).rgb);

    [loop]
    for (int y = -(int)SearchRadius; y <= (int)SearchRadius; ++y)
    {
        [loop]
        for (int x = -(int)SearchRadius; x <= (int)SearchRadius; ++x)
        {
            int2 motion = int2(x, y);
            float cost = 0.0;
            uint samples = 0;

            [loop]
            for (int py = -(int)PatchRadius; py <= (int)PatchRadius; ++py)
            {
                [loop]
                for (int px = -(int)PatchRadius; px <= (int)PatchRadius; ++px)
                {
                    int2 q = p + int2(px, py);
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
    OutFlow[p] = float2(bestMotion) * confidence;
}

// Synthesize the temporal midpoint. Flow points from the previous frame
// toward the current frame. A future host integration must handle HUD masks,
// camera cuts, frame pacing, resource barriers, and swapchain lifetime.
[numthreads(8, 8, 1)]
void SynthesizeMidpoint(uint3 tid : SV_DispatchThreadID)
{
    int2 p = int2(tid.xy);
    if (!Inside(p)) return;

    float2 motion = FlowField.Load(int3(p, 0));
    float2 halfMotion = motion * 0.5;

    float2 nowUv = (float2(p) + 0.5 - halfMotion) / float2(ImageSize);
    float2 oldUv = (float2(p) + 0.5 + halfMotion) / float2(ImageSize);

    float4 nowColor = CurrentFrame.SampleLevel(LinearClamp, nowUv, 0);
    float4 oldColor = PreviousFrame.SampleLevel(LinearClamp, oldUv, 0);
    OutFrame[p] = lerp(oldColor, nowColor, saturate(BlendFactor)) * float4(ExposureScale, ExposureScale, ExposureScale, 1.0);
}

SamplerState LinearClamp : register(s0);
