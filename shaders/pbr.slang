// ==============================================================================
// Xash3D Modern PBR Shader (Cook-Torrance GGX & Realtime Dynamic Lighting)
// Authored in Slang Shading Language
// ==============================================================================

static const float PI = 3.14159265359;
static const int MAX_LIGHTS = 16;

struct DynamicLight {
    float4 position_radius; // xyz = position, w = radius
    float4 color_intensity; // xyz = color (linear RGB), w = intensity
};

// Uniform parameters
cbuffer UniformBlock {
    float4x4 u_ModelViewProjection;
    float4x4 u_Model;
    float4x4 u_NormalMatrix;
    float4   u_ViewPosition;
    float4   u_SunDirection_Intensity; // xyz = sun dir, w = intensity
    float4   u_SunColor;               // xyz = sun color
    float4   u_AmbientColor;           // ambient light term
    float4   u_MaterialParams;         // x = metallic, y = roughness, z = ao_scale, w = exposure
    int      u_NumLights;
    DynamicLight u_Lights[MAX_LIGHTS];
};

// Textures and samplers
Sampler2D u_AlbedoMap;
Sampler2D u_NormalMap;
Sampler2D u_RoughnessMap;
Sampler2D u_LightMap;

struct VertexInput {
    float3 position : POSITION;
    float3 normal   : NORMAL;
    float4 tangent  : TANGENT;
    float2 uv0      : TEXCOORD0;
    float2 uv1      : TEXCOORD1; // lightmap coordinates
    float4 color    : COLOR;
};

struct VertexOutput {
    float4 clipPosition : SV_Position;
    float3 worldPos     : POSITION;
    float3 normal       : NORMAL;
    float3 tangent      : TANGENT;
    float3 bitangent    : BITANGENT;
    float2 uv0          : TEXCOORD0;
    float2 uv1          : TEXCOORD1;
    float4 vertexColor  : COLOR;
};

// ------------------------------------------------------------------------------
// Vertex Shader
// ------------------------------------------------------------------------------
[shader("vertex")]
VertexOutput vertexMain(VertexInput input) {
    VertexOutput output;

    float4 worldPos = mul(u_Model, float4(input.position, 1.0));
    output.worldPos = worldPos.xyz;
    output.clipPosition = mul(u_ModelViewProjection, float4(input.position, 1.0));

    // Calculate normal and TBN frame safely
    float3 N = length(input.normal) > 0.001 ? normalize(mul((float3x3)u_NormalMatrix, input.normal)) : float3(0.0, 0.0, 1.0);
    float3 T = length(input.tangent.xyz) > 0.001 ? normalize(mul((float3x3)u_NormalMatrix, input.tangent.xyz)) : float3(1.0, 0.0, 0.0);
    T = normalize(T - dot(T, N) * N);
    if (length(T) < 0.001) T = abs(N.z) < 0.999 ? normalize(cross(float3(0.0, 0.0, 1.0), N)) : normalize(cross(float3(0.0, 1.0, 0.0), N));
    float3 B = cross(N, T) * (abs(input.tangent.w) > 0.001 ? input.tangent.w : 1.0);

    output.normal = N;
    output.tangent = T;
    output.bitangent = B;
    output.uv0 = input.uv0;
    output.uv1 = input.uv1;
    output.vertexColor = input.color;

    return output;
}

// ------------------------------------------------------------------------------
// PBR Math: GGX Normal Distribution Function
// ------------------------------------------------------------------------------
float DistributionGGX(float3 N, float3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return a2 / max(denom, 0.0000001);
}

// ------------------------------------------------------------------------------
// PBR Math: Schlick-GGX Geometric Shadowing / Masking
// ------------------------------------------------------------------------------
float GeometrySchlickGGX(float NdotV, float roughness) {
    float r = (roughness + 1.0);
    float k = (r * r) / 8.0;

    float denom = NdotV * (1.0 - k) + k;
    return NdotV / max(denom, 0.0000001);
}

float GeometrySmith(float3 N, float3 V, float3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx1 = GeometrySchlickGGX(NdotV, roughness);
    float ggx2 = GeometrySchlickGGX(NdotL, roughness);

    return ggx1 * ggx2;
}

// ------------------------------------------------------------------------------
// PBR Math: Fresnel-Schlick Approximation
// ------------------------------------------------------------------------------
float3 FresnelSchlick(float cosTheta, float3 F0) {
    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}

// ------------------------------------------------------------------------------
// ACES Tone Mapping
// ------------------------------------------------------------------------------
float3 ToneMapACES(float3 x) {
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);
}

// ------------------------------------------------------------------------------
// Fragment Shader
// ------------------------------------------------------------------------------
[shader("fragment")]
float4 fragmentMain(VertexOutput input) : SV_Target {
    // Sample textures
    float4 albedoSample = u_AlbedoMap.Sample(input.uv0) * input.vertexColor;
    if (albedoSample.a < 0.1)
        discard;

    // Convert sRGB albedo to linear space
    float3 albedo = pow(albedoSample.rgb, float3(2.2, 2.2, 2.2));

    // Sample normal map (tangent space)
    float3 normalSample = u_NormalMap.Sample(input.uv0).xyz * 2.0 - 1.0;
    float3 t_norm = length(input.tangent) > 0.001 ? normalize(input.tangent) : float3(1.0, 0.0, 0.0);
    float3 b_norm = length(input.bitangent) > 0.001 ? normalize(input.bitangent) : float3(0.0, 1.0, 0.0);
    float3 n_norm = length(input.normal) > 0.001 ? normalize(input.normal) : float3(0.0, 0.0, 1.0);
    float3x3 TBN = float3x3(t_norm, b_norm, n_norm);
    float3 N = normalize(mul(TBN, normalSample));
    if (isnan(N.x) || isnan(N.y) || isnan(N.z) || length(N) < 0.001) N = n_norm;

    // View direction
    float3 V = normalize(u_ViewPosition.xyz - input.worldPos);

    // Roughness and metallic parameters
    float roughness = clamp(u_MaterialParams.y, 0.04, 1.0);
    float metallic  = clamp(u_MaterialParams.x, 0.0, 1.0);
    float ao        = clamp(u_MaterialParams.z, 0.0, 1.0);

    // Base reflectivity F0: 0.04 for dielectrics, tinted by albedo for metals
    float3 F0 = float3(0.04, 0.04, 0.04);
    F0 = lerp(F0, albedo, metallic);

    // Accumulate total outgoing radiance Lo
    float3 Lo = float3(0.0, 0.0, 0.0);

    // 1. Directional Sun Light
    float3 L_sun = normalize(-u_SunDirection_Intensity.xyz);
    float3 H_sun = normalize(V + L_sun);
    float NdotL_sun = max(dot(N, L_sun), 0.0);

    if (NdotL_sun > 0.0) {
        float3 radiance_sun = u_SunColor.rgb * u_SunDirection_Intensity.w;

        float NDF = DistributionGGX(N, H_sun, roughness);
        float G   = GeometrySmith(N, V, L_sun, roughness);
        float3 F  = FresnelSchlick(max(dot(H_sun, V), 0.0), F0);

        float3 numerator = NDF * G * F;
        float denominator = 4.0 * max(dot(N, V), 0.0) * NdotL_sun + 0.0001;
        float3 specular = numerator / denominator;

        float3 kS = F;
        float3 kD = (float3(1.0, 1.0, 1.0) - kS) * (1.0 - metallic);

        Lo += (kD * albedo / PI + specular) * radiance_sun * NdotL_sun;
    }

    // 2. Realtime Dynamic Point Lights
    int numLights = min(u_NumLights, MAX_LIGHTS);
    for (int i = 0; i < numLights; i++) {
        float3 lightPos = u_Lights[i].position_radius.xyz;
        float radius    = u_Lights[i].position_radius.w;
        float3 lightCol = u_Lights[i].color_intensity.xyz;
        float intensity = u_Lights[i].color_intensity.w;

        float3 toLight = lightPos - input.worldPos;
        float distance = length(toLight);

        if (distance >= radius || distance < 0.001)
            continue;

        float3 L = normalize(toLight);
        float3 H = normalize(V + L);

        // Smooth distance attenuation
        float atten = clamp(1.0 - (distance / radius), 0.0, 1.0);
        atten = (atten * atten) / (distance * distance * 0.01 + 1.0);
        float3 radiance = lightCol * (intensity * atten);

        float NdotL = max(dot(N, L), 0.0);
        if (NdotL <= 0.0)
            continue;

        float NDF = DistributionGGX(N, H, roughness);
        float G   = GeometrySmith(N, V, L, roughness);
        float3 F  = FresnelSchlick(max(dot(H, V), 0.0), F0);

        float3 numerator = NDF * G * F;
        float denominator = 4.0 * max(dot(N, V), 0.0) * NdotL + 0.0001;
        float3 specular = numerator / denominator;

        float3 kS = F;
        float3 kD = (float3(1.0, 1.0, 1.0) - kS) * (1.0 - metallic);

        Lo += (kD * albedo / PI + specular) * radiance * NdotL;
    }

    // 3. Ambient Lighting & Lightmaps
    float4 lmSample = u_LightMap.Sample(input.uv1);
    float3 ambient = (u_AmbientColor.rgb + lmSample.rgb * 1.5) * albedo * ao;

    // Final color before tone mapping
    float3 color = ambient + Lo;

    // Exposure and ACES tone mapping
    color *= u_MaterialParams.w; // exposure
    color = ToneMapACES(color);

    // Linear to sRGB gamma correction
    color = pow(color, float3(1.0 / 2.2, 1.0 / 2.2, 1.0 / 2.2));

    return float4(color, albedoSample.a);
}
