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
float3 ACESFilm(float3 x)
{
    float a = 2.51f;
    float b = 0.03f;
    float c = 2.43f;
    float d = 0.59f;
    float e = 0.14f;
    return saturate((x * (a * x + b)) / (x * (c * x + d) + e));
}

// ACES RRT (Reference Rendering Transform) + ODT (Output Device Transform)
// More accurate ACES implementation
static const float3x3 ACESInputMat =
{
    {0.59719, 0.35458, 0.04823},
    {0.07600, 0.90834, 0.01566},
    {0.02840, 0.13383, 0.83777}
};

static const float3x3 ACESOutputMat =
{
    { 1.60475, -0.53108, -0.07367},
    {-0.10208,  1.10813, -0.00605},
    {-0.00327, -0.07276,  1.07602}
};

float3 RRTAndODTFit(float3 v)
{
    float3 a = v * (v + 0.0245786f) - 0.000090537f;
    float3 b = v * (0.983729f * v + 0.4329510f) + 0.238081f;
    return a / b;
}

float3 ACESFitted(float3 color)
{
    color = mul(ACESInputMat, color);
    color = RRTAndODTFit(color);
    color = mul(ACESOutputMat, color);
    return color;
}

float4 main(float4 pos : SV_POSITION, float2 uv : TEXCOORD) : SV_TARGET
{
    float3 hdr = hdrTex.Sample(samp, uv).rgb;
    
    // Normalize HDR input based on max nits
    hdr = hdr * (inputMaxNits / 10000.0); // Assuming input is in [0, 10000] nits range
    
    // Apply ACES tonemapping
    // Option 1: Simple ACES (faster, good quality)
    // float3 sdr = ACESFilm(hdr);
    
    // Option 2: Full ACES fitted (slower, best quality)
    float3 sdr = ACESFitted(hdr);
    
    // Apply gamma correction for sRGB output
    sdr = pow(saturate(sdr), 1.0 / 2.2);
    
    return float4(sdr, 1.0);
}
