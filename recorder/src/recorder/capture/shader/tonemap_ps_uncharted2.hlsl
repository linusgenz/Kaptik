Texture2D<float4> hdrTex : register(t0);
SamplerState samp : register(s0);

cbuffer ToneMapCB : register(b0)
{
    float inputMaxNits;
    float outputMaxNits;
    float2 pad;
};

// Uncharted 2 Tonemapping (John Hable)
// Filmic tonemapping used in Uncharted 2
float3 Uncharted2Tonemap(float3 x)
{
    float A = 0.15;
    float B = 0.50;
    float C = 0.10;
    float D = 0.20;
    float E = 0.02;
    float F = 0.30;
    return ((x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F)) - E / F;
}

float3 Uncharted2(float3 color)
{
    float W = 11.2; // White point
    color = Uncharted2Tonemap(color * 2.0);
    float3 whiteScale = 1.0 / Uncharted2Tonemap(W);
    return color * whiteScale;
}

float4 main(float4 pos : SV_POSITION, float2 uv : TEXCOORD) : SV_TARGET
{
    float3 hdr = hdrTex.Sample(samp, uv).rgb;
    
    // Normalize HDR input based on max nits
    hdr = hdr * (inputMaxNits / 10000.0);
    
    // Apply Uncharted 2 tonemapping
    float3 sdr = Uncharted2(hdr);
    
    // Apply gamma correction for sRGB output
    sdr = pow(saturate(sdr), 1.0 / 2.2);
    
    return float4(sdr, 1.0);
}
