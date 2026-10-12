// ==============================================================================
// Xash3D FXAA 3.11 Anti-Aliasing Post-Process Shader
// Authored in Slang Shading Language
// ==============================================================================

cbuffer FXAAUniformBlock {
    float4 u_ScreenResolution; // xy = resolution (w, h), zw = rcp resolution (1/w, 1/h)
};

Sampler2D u_ColorMap;

struct VertexInput {
    float3 position : POSITION;
    float2 uv       : TEXCOORD0;
};

struct VertexOutput {
    float4 clipPosition : SV_Position;
    float2 uv           : TEXCOORD0;
};

[shader("vertex")]
VertexOutput vertexMain(VertexInput input) {
    VertexOutput output;
    output.clipPosition = float4(input.position.xy, 0.0, 1.0);
    output.uv = input.uv;
    return output;
}

// Convert linear or sRGB to perceptual luminance
float FxaaLuma(float3 rgb) {
    return dot(rgb, float3(0.299, 0.587, 0.114));
}

static const float FXAA_EDGE_THRESHOLD_MIN = 0.0312;
static const float FXAA_EDGE_THRESHOLD     = 0.125;
static const float FXAA_SUBPIX_TRIM        = 0.25;
static const float FXAA_SUBPIX_CAP         = 0.75;
static const int   FXAA_SEARCH_STEPS       = 12;

[shader("fragment")]
float4 fragmentMain(VertexOutput input) : SV_Target {
    float2 uv = input.uv;
    float2 rcpFrame = u_ScreenResolution.zw;

    // Sample center and 4 direct neighbors
    float3 rgbM = u_ColorMap.Sample(uv).rgb;
    float lumaM = FxaaLuma(rgbM);

    float lumaN = FxaaLuma(u_ColorMap.Sample(uv + float2(0.0, -rcpFrame.y)).rgb);
    float lumaS = FxaaLuma(u_ColorMap.Sample(uv + float2(0.0,  rcpFrame.y)).rgb);
    float lumaW = FxaaLuma(u_ColorMap.Sample(uv + float2(-rcpFrame.x, 0.0)).rgb);
    float lumaE = FxaaLuma(u_ColorMap.Sample(uv + float2( rcpFrame.x, 0.0)).rgb);

    float lumaMin = min(lumaM, min(min(lumaN, lumaS), min(lumaW, lumaE)));
    float lumaMax = max(lumaM, max(max(lumaN, lumaS), max(lumaW, lumaE)));
    float range = lumaMax - lumaMin;

    // If local contrast is below threshold, skip AA
    if (range < max(FXAA_EDGE_THRESHOLD_MIN, lumaMax * FXAA_EDGE_THRESHOLD)) {
        return float4(rgbM, 1.0);
    }

    // Sample diagonals for edge orientation and subpixel filtering
    float lumaNW = FxaaLuma(u_ColorMap.Sample(uv + float2(-rcpFrame.x, -rcpFrame.y)).rgb);
    float lumaNE = FxaaLuma(u_ColorMap.Sample(uv + float2( rcpFrame.x, -rcpFrame.y)).rgb);
    float lumaSW = FxaaLuma(u_ColorMap.Sample(uv + float2(-rcpFrame.x,  rcpFrame.y)).rgb);
    float lumaSE = FxaaLuma(u_ColorMap.Sample(uv + float2( rcpFrame.x,  rcpFrame.y)).rgb);

    // Sub-pixel quality evaluation
    float lumaL = (lumaN + lumaS + lumaW + lumaE) * 0.25;
    float rangeL = abs(lumaL - lumaM);
    float blendL = max(0.0, (rangeL / range) - FXAA_SUBPIX_TRIM) * (1.0 / (1.0 - FXAA_SUBPIX_TRIM));
    blendL = min(FXAA_SUBPIX_CAP, blendL);

    // Determine edge direction: Horizontal vs Vertical
    float edgeHorz = abs(-2.0 * lumaW + lumaNW + lumaSW) +
                     2.0 * abs(-2.0 * lumaM + lumaN + lumaS) +
                     abs(-2.0 * lumaE + lumaNE + lumaSE);
    float edgeVert = abs(-2.0 * lumaN + lumaNW + lumaNE) +
                     2.0 * abs(-2.0 * lumaM + lumaW + lumaE) +
                     abs(-2.0 * lumaS + lumaSW + lumaSE);
    bool isHorz = edgeHorz >= edgeVert;

    // Select step direction
    float lengthSign = isHorz ? rcpFrame.y : rcpFrame.x;
    float luma1 = isHorz ? lumaN : lumaW;
    float luma2 = isHorz ? lumaS : lumaE;
    float gradient1 = abs(luma1 - lumaM);
    float gradient2 = abs(luma2 - lumaM);

    if (gradient1 < gradient2) {
        lengthSign = -lengthSign;
        luma1 = luma2;
    }

    float2 posN = uv;
    if (isHorz)
        posN.y += lengthSign * 0.5;
    else
        posN.x += lengthSign * 0.5;

    // Edge endpoints search
    float2 stepDir = isHorz ? float2(rcpFrame.x, 0.0) : float2(0.0, rcpFrame.y);
    float2 posP = posN + stepDir;
    posN -= stepDir;

    float lumaEndN = FxaaLuma(u_ColorMap.Sample(posN).rgb) - luma1 * 0.5;
    float lumaEndP = FxaaLuma(u_ColorMap.Sample(posP).rgb) - luma1 * 0.5;
    bool doneN = abs(lumaEndN) >= gradient1 * 0.5;
    bool doneP = abs(lumaEndP) >= gradient1 * 0.5;

    for (int i = 0; i < FXAA_SEARCH_STEPS && (!doneN || !doneP); i++) {
        if (!doneN) posN -= stepDir;
        if (!doneP) posP += stepDir;
        if (!doneN) lumaEndN = FxaaLuma(u_ColorMap.Sample(posN).rgb) - luma1 * 0.5;
        if (!doneP) lumaEndP = FxaaLuma(u_ColorMap.Sample(posP).rgb) - luma1 * 0.5;
        doneN = abs(lumaEndN) >= gradient1 * 0.5;
        doneP = abs(lumaEndP) >= gradient1 * 0.5;
    }

    float distN = isHorz ? (uv.x - posN.x) : (uv.y - posN.y);
    float distP = isHorz ? (posP.x - uv.x) : (posP.y - uv.y);
    bool directionN = distN < distP;
    float dist = min(distN, distP);
    float span = distN + distP;
    float pixelOffset = -dist / span + 0.5;

    float2 finalUV = uv;
    if (isHorz)
        finalUV.y += pixelOffset * lengthSign;
    else
        finalUV.x += pixelOffset * lengthSign;

    float3 rgbFinal = u_ColorMap.Sample(finalUV).rgb;
    rgbFinal = lerp(rgbFinal, rgbM, blendL);

    return float4(rgbFinal, 1.0);
}
