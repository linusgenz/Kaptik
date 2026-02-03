Texture2D<float4> hdrTex : register(t0);
SamplerState samp : register(s0);

cbuffer ToneMapCB : register(b0)
{
    float inputMaxNits;
    float outputMaxNits;
    float2 pad;
};

// ACES Filmic Tone Mapping Curve
// Narkowicz 2015, "ACES Filmic Tone Mapping Curve"
// Fast approximation, good quality
float3 ACESFilm(float3 x)
{
    float a = 2.51f;
    float b = 0.03f;
    float c = 2.43f;
    float d = 0.59f;
    float e = 0.14f;
    return saturate((x * (a * x + b)) / (x * (c * x + d) + e));
}

float4 main(float4 pos : SV_POSITION, float2 uv : TEXCOORD) : SV_TARGET
{
    float3 hdr = hdrTex.Sample(samp, uv).rgb;
    
    // Normalize HDR input based on max nits
    hdr = hdr * (inputMaxNits / 10000.0);
    
    // Apply ACES Simple tonemapping
    float3 sdr = ACESFilm(hdr);
    
    // Apply gamma correction for sRGB output
    sdr = pow(saturate(sdr), 1.0 / 2.2);
    
    return float4(sdr, 1.0);
}
