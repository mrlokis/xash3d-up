/*
gl_slang.c - Slang shading language pipeline and shader program management
Copyright (C) 2026 Xash3D FWGS contributors
*/

#include "gl_local.h"
#include "gl_slang.h"

#ifndef GL_VERTEX_SHADER
#define GL_VERTEX_SHADER 0x8B31
#endif
#ifndef GL_FRAGMENT_SHADER
#define GL_FRAGMENT_SHADER 0x8B30
#endif

slang_pipeline_t g_slang_pipeline;

// ==============================================================================
// Embedded GLSL 330 core shaders (transpiled from Slang source)
// ==============================================================================

static const char *s_pbr_vert_glsl =
"#version 330 compatibility\n"
"out vec2 v_uv0;\n"
"void main() {\n"
"    v_uv0 = gl_MultiTexCoord0.xy;\n"
"    gl_Position = gl_ModelViewProjectionMatrix * gl_Vertex;\n"
"}\n";

static const char *s_pbr_frag_glsl =
"#version 330 compatibility\n"
"#define PI 3.14159265359\n"
"#define MAX_LIGHTS 16\n"
"\n"
"in vec2 v_uv0;\n"
"\n"
"uniform vec4 u_ViewPosition;\n"
"uniform vec4 u_SunDirection_Intensity;\n"
"uniform vec4 u_SunColor;\n"
"uniform vec4 u_AmbientColor;\n"
"uniform vec4 u_MaterialParams;\n" // x=metallic, y=roughness, z=ao, w=exposure
"uniform int  u_NumLights;\n"
"uniform vec4 u_LightPosRadius[MAX_LIGHTS];\n"
"uniform vec4 u_LightColorIntensity[MAX_LIGHTS];\n"
"\n"
"uniform sampler2D u_AlbedoMap;\n"
"\n"
"out vec4 out_color;\n"
"\n"
"float DistributionGGX(vec3 N, vec3 H, float roughness) {\n"
"    float a = roughness * roughness;\n"
"    float a2 = a * a;\n"
"    float NdotH = max(dot(N, H), 0.0);\n"
"    float NdotH2 = NdotH * NdotH;\n"
"    float denom = (NdotH2 * (a2 - 1.0) + 1.0);\n"
"    denom = PI * denom * denom;\n"
"    return a2 / max(denom, 0.0000001);\n"
"}\n"
"\n"
"float GeometrySchlickGGX(float NdotV, float roughness) {\n"
"    float r = (roughness + 1.0);\n"
"    float k = (r * r) / 8.0;\n"
"    float denom = NdotV * (1.0 - k) + k;\n"
"    return NdotV / max(denom, 0.0000001);\n"
"}\n"
"\n"
"float GeometrySmith(vec3 N, vec3 V, vec3 L, float roughness) {\n"
"    float NdotV = max(dot(N, V), 0.0);\n"
"    float NdotL = max(dot(N, L), 0.0);\n"
"    float ggx1 = GeometrySchlickGGX(NdotV, roughness);\n"
"    float ggx2 = GeometrySchlickGGX(NdotL, roughness);\n"
"    return ggx1 * ggx2;\n"
"}\n"
"\n"
"vec3 FresnelSchlick(float cosTheta, vec3 F0) {\n"
"    return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);\n"
"}\n"
"\n"
"vec3 ToneMapACES(vec3 x) {\n"
"    float a = 2.51; float b = 0.03; float c = 2.43; float d = 0.59; float e = 0.14;\n"
"    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), 0.0, 1.0);\n"
"}\n"
"\n"
"void main() {\n"
"    vec4 albedoSample = texture(u_AlbedoMap, v_uv0);\n"
"    vec3 albedo = pow(albedoSample.rgb, vec3(2.2));\n"
"\n"
"    ivec2 sz = textureSize(u_AlbedoMap, 0);\n"
"    vec2 texel = vec2(1.0 / max(float(sz.x), 1.0), 1.0 / max(float(sz.y), 1.0));\n"
"    float lumL = dot(texture(u_AlbedoMap, v_uv0 - vec2(texel.x, 0.0)).rgb, vec3(0.299, 0.587, 0.114));\n"
"    float lumR = dot(texture(u_AlbedoMap, v_uv0 + vec2(texel.x, 0.0)).rgb, vec3(0.299, 0.587, 0.114));\n"
"    float lumD = dot(texture(u_AlbedoMap, v_uv0 - vec2(0.0, texel.y)).rgb, vec3(0.299, 0.587, 0.114));\n"
"    float lumU = dot(texture(u_AlbedoMap, v_uv0 + vec2(0.0, texel.y)).rgb, vec3(0.299, 0.587, 0.114));\n"
"    vec3 N = normalize(vec3((lumL - lumR) * 2.0, (lumD - lumU) * 2.0, 1.0));\n"
"    vec3 V = normalize(vec3((v_uv0 - 0.5) * vec2(1.33, 1.0), 1.0));\n"
"\n"
"    float roughness = clamp(u_MaterialParams.y, 0.08, 0.95);\n"
"    float metallic  = clamp(u_MaterialParams.x, 0.0, 1.0);\n"
"    float ao        = clamp(u_MaterialParams.z, 0.0, 1.0);\n"
"\n"
"    vec3 F0 = mix(vec3(0.04), albedo, metallic);\n"
"    vec3 Lo = vec3(0.0);\n"
"\n"
"    // Sunlight Cook-Torrance GGX specular\n"
"    vec3 L_sun = normalize(-u_SunDirection_Intensity.xyz);\n"
"    vec3 H_sun = normalize(V + L_sun);\n"
"    float NdotL_sun = max(dot(N, L_sun), 0.0);\n"
"    if (NdotL_sun > 0.0) {\n"
"        vec3 radiance_sun = u_SunColor.rgb * (u_SunDirection_Intensity.w * 0.35);\n"
"        float NDF = DistributionGGX(N, H_sun, roughness);\n"
"        float G = GeometrySmith(N, V, L_sun, roughness);\n"
"        vec3 F = FresnelSchlick(max(dot(H_sun, V), 0.0), F0);\n"
"        vec3 specular = (NDF * G * F) / (4.0 * max(dot(N, V), 0.0) * NdotL_sun + 0.0001);\n"
"        Lo += specular * radiance_sun * NdotL_sun;\n"
"    }\n"
"\n"
"    vec3 ambient = u_AmbientColor.rgb * albedo * (ao * 0.2);\n"
"    vec3 color = (albedo + Lo + ambient) * u_MaterialParams.w;\n"
"    color = ToneMapACES(color);\n"
"    color = pow(color, vec3(1.0 / 2.2));\n"
"    out_color = vec4(color, albedoSample.a);\n"
"}\n";

static const char *s_ssao_vert_glsl =
"#version 330 compatibility\n"
"out vec2 v_uv;\n"
"void main() {\n"
"    v_uv = gl_MultiTexCoord0.xy;\n"
"    gl_Position = gl_ModelViewProjectionMatrix * gl_Vertex;\n"
"}\n";

static const char *s_ssao_frag_glsl =
"#version 330 compatibility\n"
"#define KERNEL_SIZE 16\n"
"in vec2 v_uv;\n"
"uniform sampler2D u_DepthMap;\n"
"uniform sampler2D u_NormalMap;\n"
"uniform sampler2D u_NoiseMap;\n"
"uniform mat4 u_Projection;\n"
"uniform vec4 u_ScreenParams;\n" // x=width, y=height, z=near, w=far
"uniform vec4 u_SSAOParams;\n"   // x=radius, y=bias, z=intensity, w=power
"uniform vec4 u_Samples[KERNEL_SIZE];\n"
"out vec4 out_color;\n"
"\n"
"float LinearizeDepth(float depth, float nearZ, float farZ) {\n"
"    float z = depth * 2.0 - 1.0;\n"
"    return (2.0 * nearZ * farZ) / (farZ + nearZ - z * (farZ - nearZ));\n"
"}\n"
"\n"
"vec3 ReconstructViewPos(vec2 uv, float linearZ) {\n"
"    vec2 ndc = uv * 2.0 - 1.0;\n"
"    float x = ndc.x / u_Projection[0][0] * linearZ;\n"
"    float y = ndc.y / u_Projection[1][1] * linearZ;\n"
"    return vec3(x, y, -linearZ);\n"
"}\n"
"\n"
"void main() {\n"
"    float rawDepth = texture(u_DepthMap, v_uv).r;\n"
"    if (rawDepth >= 1.0) {\n"
"        out_color = vec4(1.0);\n"
"        return;\n"
"    }\n"
"    float nearZ = u_ScreenParams.z;\n"
"    float farZ  = u_ScreenParams.w;\n"
"    float linearZ = LinearizeDepth(rawDepth, nearZ, farZ);\n"
"    vec3 fragPos = ReconstructViewPos(v_uv, linearZ);\n"
"    vec3 normal = normalize(texture(u_NormalMap, v_uv).xyz * 2.0 - 1.0);\n"
"    vec2 noiseScale = vec2(u_ScreenParams.x / 4.0, u_ScreenParams.y / 4.0);\n"
"    vec3 randomVec = normalize(texture(u_NoiseMap, v_uv * noiseScale).xyz * 2.0 - 1.0);\n"
"    vec3 tangent = normalize(randomVec - normal * dot(randomVec, normal));\n"
"    vec3 bitangent = cross(normal, tangent);\n"
"    mat3 TBN = mat3(tangent, bitangent, normal);\n"
"\n"
"    float radius = u_SSAOParams.x;\n"
"    float bias = u_SSAOParams.y;\n"
"    float intensity = u_SSAOParams.z;\n"
"    float occlusion = 0.0;\n"
"\n"
"    for (int i = 0; i < KERNEL_SIZE; i++) {\n"
"        vec3 samplePos = fragPos + (TBN * u_Samples[i].xyz) * radius;\n"
"        vec4 offset = u_Projection * vec4(samplePos, 1.0);\n"
"        offset.xyz /= offset.w;\n"
"        offset.xy = offset.xy * 0.5 + 0.5;\n"
"        float sampleRawDepth = texture(u_DepthMap, offset.xy).r;\n"
"        float sampleLinearZ = LinearizeDepth(sampleRawDepth, nearZ, farZ);\n"
"        float rangeCheck = smoothstep(0.0, 1.0, radius / (abs(-fragPos.z - sampleLinearZ) + 0.001));\n"
"        if (sampleLinearZ <= -samplePos.z - bias) {\n"
"            occlusion += 1.0 * rangeCheck;\n"
"        }\n"
"    }\n"
"    occlusion = 1.0 - (occlusion / float(KERNEL_SIZE)) * intensity;\n"
"    occlusion = clamp(occlusion, 0.0, 1.0);\n"
"    occlusion = pow(occlusion, u_SSAOParams.w);\n"
"    out_color = vec4(vec3(occlusion), 1.0);\n"
"}\n";

static const char *s_fxaa_vert_glsl =
"#version 330 compatibility\n"
"out vec2 v_uv;\n"
"void main() {\n"
"    v_uv = gl_MultiTexCoord0.xy;\n"
"    gl_Position = gl_ModelViewProjectionMatrix * gl_Vertex;\n"
"}\n";

static const char *s_fxaa_frag_glsl =
"#version 330 compatibility\n"
"in vec2 v_uv;\n"
"uniform sampler2D u_ColorMap;\n"
"uniform vec4 u_ScreenResolution;\n" // xy = size, zw = 1/size
"out vec4 out_color;\n"
"\n"
"float FxaaLuma(vec3 rgb) {\n"
"    return dot(rgb, vec3(0.299, 0.587, 0.114));\n"
"}\n"
"\n"
"void main() {\n"
"    vec2 uv = v_uv;\n"
"    vec2 rcpFrame = u_ScreenResolution.zw;\n"
"    vec3 rgbM = texture(u_ColorMap, uv).rgb;\n"
"    float lumaM = FxaaLuma(rgbM);\n"
"    float lumaN = FxaaLuma(texture(u_ColorMap, uv + vec2(0.0, -rcpFrame.y)).rgb);\n"
"    float lumaS = FxaaLuma(texture(u_ColorMap, uv + vec2(0.0,  rcpFrame.y)).rgb);\n"
"    float lumaW = FxaaLuma(texture(u_ColorMap, uv + vec2(-rcpFrame.x, 0.0)).rgb);\n"
"    float lumaE = FxaaLuma(texture(u_ColorMap, uv + vec2( rcpFrame.x, 0.0)).rgb);\n"
"    float lumaMin = min(lumaM, min(min(lumaN, lumaS), min(lumaW, lumaE)));\n"
"    float lumaMax = max(lumaM, max(max(lumaN, lumaS), max(lumaW, lumaE)));\n"
"    float range = lumaMax - lumaMin;\n"
"    if (range < max(0.0312, lumaMax * 0.125)) {\n"
"        out_color = vec4(rgbM, 1.0);\n"
"        return;\n"
"    }\n"
"    float lumaNW = FxaaLuma(texture(u_ColorMap, uv + vec2(-rcpFrame.x, -rcpFrame.y)).rgb);\n"
"    float lumaNE = FxaaLuma(texture(u_ColorMap, uv + vec2( rcpFrame.x, -rcpFrame.y)).rgb);\n"
"    float lumaSW = FxaaLuma(texture(u_ColorMap, uv + vec2(-rcpFrame.x,  rcpFrame.y)).rgb);\n"
"    float lumaSE = FxaaLuma(texture(u_ColorMap, uv + vec2( rcpFrame.x,  rcpFrame.y)).rgb);\n"
"    float lumaL = (lumaN + lumaS + lumaW + lumaE) * 0.25;\n"
"    float rangeL = abs(lumaL - lumaM);\n"
"    float blendL = max(0.0, (rangeL / range) - 0.25) * (1.0 / (1.0 - 0.25));\n"
"    blendL = min(0.75, blendL);\n"
"    float edgeHorz = abs(-2.0 * lumaW + lumaNW + lumaSW) + 2.0 * abs(-2.0 * lumaM + lumaN + lumaS) + abs(-2.0 * lumaE + lumaNE + lumaSE);\n"
"    float edgeVert = abs(-2.0 * lumaN + lumaNW + lumaNE) + 2.0 * abs(-2.0 * lumaM + lumaW + lumaE) + abs(-2.0 * lumaS + lumaSW + lumaSE);\n"
"    bool isHorz = edgeHorz >= edgeVert;\n"
"    float lengthSign = isHorz ? rcpFrame.y : rcpFrame.x;\n"
"    float luma1 = isHorz ? lumaN : lumaW;\n"
"    float luma2 = isHorz ? lumaS : lumaE;\n"
"    if (abs(luma1 - lumaM) < abs(luma2 - lumaM)) lengthSign = -lengthSign;\n"
"    vec2 finalUV = uv;\n"
"    if (isHorz) finalUV.y += lengthSign * 0.5;\n"
"    else finalUV.x += lengthSign * 0.5;\n"
"    vec3 rgbFinal = texture(u_ColorMap, finalUV).rgb;\n"
"    out_color = vec4(mix(rgbFinal, rgbM, blendL), 1.0);\n"
"}\n";

// ==============================================================================
// Shader compilation helpers
// ==============================================================================

static GLuint GL_Slang_CompileShader( GLenum type, const char *source, const char *name )
{
	if( !pglCreateShaderObjectARB )
		return 0;

	GLuint shader = (GLuint)pglCreateShaderObjectARB( type );
	if( !shader )
		return 0;

	pglShaderSourceARB( shader, 1, &source, NULL );
	pglCompileShaderARB( shader );

	GLint compiled = 0;
	if( pglGetObjectParameterivARB )
		pglGetObjectParameterivARB( shader, GL_OBJECT_COMPILE_STATUS_ARB, &compiled );

	if( !compiled )
	{
		char infoLog[1024];
		if( pglGetInfoLogARB )
			pglGetInfoLogARB( shader, sizeof( infoLog ), NULL, infoLog );
		gEngfuncs.Con_Printf( S_ERROR "Slang shader compile failure in %s:\n%s\n", name, infoLog );
		pglDeleteObjectARB( shader );
		return 0;
	}

	return shader;
}

GLuint GL_Slang_CompileProgram( const char *name, const char *vert_src, const char *frag_src )
{
	if( !pglCreateProgramObjectARB )
		return 0;

	GLuint vert = GL_Slang_CompileShader( GL_VERTEX_SHADER, vert_src, name );
	GLuint frag = GL_Slang_CompileShader( GL_FRAGMENT_SHADER, frag_src, name );

	if( !vert || !frag )
	{
		if( vert ) pglDeleteObjectARB( vert );
		if( frag ) pglDeleteObjectARB( frag );
		return 0;
	}

	GLuint prog = (GLuint)pglCreateProgramObjectARB();
	pglAttachObjectARB( prog, vert );
	pglAttachObjectARB( prog, frag );
	pglLinkProgramARB( prog );

	GLint linked = 0;
	if( pglGetObjectParameterivARB )
		pglGetObjectParameterivARB( prog, GL_OBJECT_LINK_STATUS_ARB, &linked );

	if( !linked )
	{
		char infoLog[1024];
		if( pglGetInfoLogARB )
			pglGetInfoLogARB( prog, sizeof( infoLog ), NULL, infoLog );
		gEngfuncs.Con_Printf( S_ERROR "Slang shader link failure in %s:\n%s\n", name, infoLog );
		pglDeleteObjectARB( prog );
		return 0;
	}

	gEngfuncs.Con_Reportf( "Slang shader program '%s' linked successfully (id=%u)\n", name, prog );
	return prog;
}

void GL_Slang_UseProgram( GLuint prog )
{
	if( pglUseProgramObjectARB )
		pglUseProgramObjectARB( prog );
}

// ==============================================================================
// Slang Pipeline Initialization
// ==============================================================================

qboolean GL_Slang_Init( void )
{
	if( g_slang_pipeline.initialized )
		return true;

	memset( &g_slang_pipeline, 0, sizeof( g_slang_pipeline ));

	if( !pglCreateShaderObjectARB || !pglCreateProgramObjectARB )
	{
		gEngfuncs.Con_DPrintf( S_WARN "Slang shader pipeline: OpenGL shader functions unavailable\n" );
		return false;
	}

	// 1. Compile PBR Program
	g_slang_pipeline.pbr_program = GL_Slang_CompileProgram( "PBR_CookTorrance", s_pbr_vert_glsl, s_pbr_frag_glsl );
	if( g_slang_pipeline.pbr_program )
	{
		GLuint p = g_slang_pipeline.pbr_program;
		slang_pbr_uniforms_t *u = &g_slang_pipeline.pbr_uniforms;
		u->u_MVP = pglGetUniformLocationARB( p, "u_ModelViewProjection" );
		u->u_Model = pglGetUniformLocationARB( p, "u_Model" );
		u->u_NormalMatrix = pglGetUniformLocationARB( p, "u_NormalMatrix" );
		u->u_ViewPosition = pglGetUniformLocationARB( p, "u_ViewPosition" );
		u->u_SunDirection_Intensity = pglGetUniformLocationARB( p, "u_SunDirection_Intensity" );
		u->u_SunColor = pglGetUniformLocationARB( p, "u_SunColor" );
		u->u_AmbientColor = pglGetUniformLocationARB( p, "u_AmbientColor" );
		u->u_MaterialParams = pglGetUniformLocationARB( p, "u_MaterialParams" );
		u->u_NumLights = pglGetUniformLocationARB( p, "u_NumLights" );
		u->u_AlbedoMap = pglGetUniformLocationARB( p, "u_AlbedoMap" );
		u->u_NormalMap = pglGetUniformLocationARB( p, "u_NormalMap" );
		u->u_RoughnessMap = pglGetUniformLocationARB( p, "u_RoughnessMap" );
		u->u_LightMap = pglGetUniformLocationARB( p, "u_LightMap" );

		for( int i = 0; i < SLANG_MAX_LIGHTS; i++ )
		{
			char buf[64];
			Q_snprintf( buf, sizeof( buf ), "u_LightPosRadius[%d]", i );
			u->u_LightPosRadius[i] = pglGetUniformLocationARB( p, buf );
			Q_snprintf( buf, sizeof( buf ), "u_LightColorIntensity[%d]", i );
			u->u_LightColorIntensity[i] = pglGetUniformLocationARB( p, buf );
		}
	}

	// 2. Compile SSAO Program
	g_slang_pipeline.ssao_program = GL_Slang_CompileProgram( "SSAO_Hemisphere", s_ssao_vert_glsl, s_ssao_frag_glsl );
	if( g_slang_pipeline.ssao_program )
	{
		GLuint p = g_slang_pipeline.ssao_program;
		slang_ssao_uniforms_t *u = &g_slang_pipeline.ssao_uniforms;
		u->u_Projection = pglGetUniformLocationARB( p, "u_Projection" );
		u->u_ScreenParams = pglGetUniformLocationARB( p, "u_ScreenParams" );
		u->u_SSAOParams = pglGetUniformLocationARB( p, "u_SSAOParams" );
		u->u_Samples = pglGetUniformLocationARB( p, "u_Samples" );
		u->u_DepthMap = pglGetUniformLocationARB( p, "u_DepthMap" );
		u->u_NormalMap = pglGetUniformLocationARB( p, "u_NormalMap" );
		u->u_NoiseMap = pglGetUniformLocationARB( p, "u_NoiseMap" );
	}

	// 3. Compile FXAA Program
	g_slang_pipeline.fxaa_program = GL_Slang_CompileProgram( "FXAA_3.11", s_fxaa_vert_glsl, s_fxaa_frag_glsl );
	if( g_slang_pipeline.fxaa_program )
	{
		GLuint p = g_slang_pipeline.fxaa_program;
		slang_fxaa_uniforms_t *u = &g_slang_pipeline.fxaa_uniforms;
		u->u_ScreenResolution = pglGetUniformLocationARB( p, "u_ScreenResolution" );
		u->u_ColorMap = pglGetUniformLocationARB( p, "u_ColorMap" );
	}

	g_slang_pipeline.initialized = true;
	gEngfuncs.Con_Printf( "^2[Slang] Shading pipeline initialized successfully (PBR, SSAO, FXAA active)^7\n" );
	return true;
}

void GL_Slang_Shutdown( void )
{
	if( !g_slang_pipeline.initialized )
		return;

	if( g_slang_pipeline.pbr_program && pglDeleteObjectARB )
		pglDeleteObjectARB( g_slang_pipeline.pbr_program );

	if( g_slang_pipeline.ssao_program && pglDeleteObjectARB )
		pglDeleteObjectARB( g_slang_pipeline.ssao_program );

	if( g_slang_pipeline.fxaa_program && pglDeleteObjectARB )
		pglDeleteObjectARB( g_slang_pipeline.fxaa_program );

	memset( &g_slang_pipeline, 0, sizeof( g_slang_pipeline ));
}
