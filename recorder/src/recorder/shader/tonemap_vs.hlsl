struct VSOut { float4 pos : SV_POSITION; float2 uv : TEXCOORD0; };

VSOut main(uint id : SV_VertexID)
{
    float2 pos[3] = { {-1,-1}, {-1,3}, {3,-1} };
    float2 uv[3]  = { {0,1}, {0,-1}, {2,1} };
    VSOut o;
    o.pos = float4(pos[id],0,1);
    o.uv = uv[id];
    return o;
}
