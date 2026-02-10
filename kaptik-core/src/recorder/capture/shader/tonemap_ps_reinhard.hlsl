Texture2D<float4> hdrTex : register(t0);
SamplerState samp : register(s0);

cbuffer ToneMapCB : register(b0)
{
    float inputMaxNits;
    float outputMaxNits;
    float exposure;
    float2 pad;
};

float3 Reinhard(float3 c)
{
    c = c / inputMaxNits;
    c = c / (1.0 + c);
    c = c * outputMaxNits;
    return c;
}

float4 main(float4 pos : SV_POSITION, float2 uv : TEXCOORD) : SV_TARGET
{
    float3 hdr = hdrTex.Sample(samp, uv).rgb;

    float3 scene = hdr / inputMaxNits;
    scene *= exposure;
    float3 sdr = Reinhard(scene);
    sdr *= outputMaxNits;
    sdr = pow(saturate(sdr), 1.0/2.2);

    return float4(sdr, 1.0);
}
