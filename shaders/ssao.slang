// ==============================================================================
// Xash3D Screen-Space Ambient Occlusion (SSAO) Shader
// Authored in Slang Shading Language
// ==============================================================================

static const int KERNEL_SIZE = 16;

cbuffer SSAOUniformBlock {
    float4x4 u_Projection;
    float4x4 u_InvProjection;
    float4x4 u_View;
    float4   u_ScreenParams; // x = width, y = height, z = near, w = far
    float4   u_SSAOParams;   // x = radius, y = bias, z = intensity, w = power
    float4   u_Samples[KERNEL_SIZE];
};

Sampler2D u_DepthMap;
Sampler2D u_NormalMap;
Sampler2D u_NoiseMap;

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

// Convert depth buffer value to view-space linear Z
float LinearizeDepth(float depth, float nearZ, float farZ) {
    float z = depth * 2.0 - 1.0;
    return (2.0 * nearZ * farZ) / (farZ + nearZ - z * (farZ - nearZ));
}

// Reconstruct view-space position from UV and depth
float3 ReconstructViewPos(float2 uv, float linearZ) {
    float2 ndc = uv * 2.0 - 1.0;
    float x = ndc.x / u_Projection[0][0] * linearZ;
    float y = ndc.y / u_Projection[1][1] * linearZ;
    return float3(x, y, -linearZ);
}

[shader("fragment")]
float4 fragmentMain(VertexOutput input) : SV_Target {
    float rawDepth = u_DepthMap.Sample(input.uv).r;
    if (rawDepth >= 1.0)
        return float4(1.0, 1.0, 1.0, 1.0); // Skybox has no occlusion

    float nearZ = u_ScreenParams.z;
    float farZ  = u_ScreenParams.w;
    float linearZ = LinearizeDepth(rawDepth, nearZ, farZ);
    float3 fragPos = ReconstructViewPos(input.uv, linearZ);

    // Normal in view space
    float3 normal = u_NormalMap.Sample(input.uv).xyz * 2.0 - 1.0;
    normal = normalize(normal);

    // Random noise tile
    float2 noiseScale = float2(u_ScreenParams.x / 4.0, u_ScreenParams.y / 4.0);
    float3 randomVec = u_NoiseMap.Sample(input.uv * noiseScale).xyz * 2.0 - 1.0;

    // Create TBN change-of-basis matrix for hemisphere
    float3 tangent   = normalize(randomVec - normal * dot(randomVec, normal));
    float3 bitangent = cross(normal, tangent);
    float3x3 TBN     = float3x3(tangent, bitangent, normal);

    float radius    = u_SSAOParams.x;
    float bias      = u_SSAOParams.y;
    float intensity = u_SSAOParams.z;

    float occlusion = 0.0;
    for (int i = 0; i < KERNEL_SIZE; i++) {
        // Reorient sample from tangent space to view space
        float3 samplePos = mul(TBN, u_Samples[i].xyz);
        samplePos = fragPos + samplePos * radius;

        // Project sample position to screen space UV
        float4 offset = float4(samplePos, 1.0);
        offset = mul(u_Projection, offset);
        offset.xyz /= offset.w;
        offset.xy = offset.xy * 0.5 + 0.5;

        // Sample depth at offset
        float sampleRawDepth = u_DepthMap.Sample(offset.xy).r;
        float sampleLinearZ  = LinearizeDepth(sampleRawDepth, nearZ, farZ);

        // Range check and accumulate
        float rangeCheck = smoothstep(0.0, 1.0, radius / (abs(-fragPos.z - sampleLinearZ) + 0.001));
        if (sampleLinearZ <= -samplePos.z - bias) {
            occlusion += 1.0 * rangeCheck;
        }
    }

    occlusion = 1.0 - (occlusion / float(KERNEL_SIZE)) * intensity;
    occlusion = clamp(occlusion, 0.0, 1.0);
    occlusion = pow(occlusion, u_SSAOParams.w);

    return float4(occlusion, occlusion, occlusion, 1.0);
}
