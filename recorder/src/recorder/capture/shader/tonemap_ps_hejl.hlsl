Texture2D<float4> hdrTex : register(t0);
SamplerState samp : register(s0);

cbuffer ToneMapCB : register(b0)
{
    float inputMaxNits;
    float outputMaxNits;
    float2 pad;
};

// Hejl-Dawson Tonemapping
// Simple and fast filmic tonemapping
float3 HejlDawson(float3 color)
{
    color = max(float3(0, 0, 0), color - 0.004);
    color = (color * (6.2 * color + 0.5)) / (color * (6.2 * color + 1.7) + 0.06);
    return color;
}

float4 main(float4 pos : SV_POSITION, float2 uv : TEXCOORD) : SV_TARGET
{
    float3 hdr = hdrTex.Sample(samp, uv).rgb;
    
    // Normalize HDR input based on max nits
    hdr = hdr * (inputMaxNits / 10000.0);
    
    // Apply Hejl-Dawson tonemapping
    // Note: This operator includes its own gamma correction
    float3 sdr = HejlDawson(hdr);
    
    return float4(sdr, 1.0);
}
