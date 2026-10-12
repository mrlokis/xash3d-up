/*
gl_slang.h - Slang shading language runtime and shader program interface
Copyright (C) 2026 Xash3D FWGS contributors
*/

#ifndef GL_SLANG_H
#define GL_SLANG_H

#include "gl_local.h"

#define SLANG_MAX_LIGHTS 16

typedef struct
{
	vec4_t position_radius; // xyz = world pos, w = radius
	vec4_t color_intensity; // xyz = color RGB, w = intensity
} slang_light_t;

typedef struct slang_pbr_uniforms_s
{
	GLint u_MVP;
	GLint u_Model;
	GLint u_NormalMatrix;
	GLint u_ViewPosition;
	GLint u_SunDirection_Intensity;
	GLint u_SunColor;
	GLint u_AmbientColor;
	GLint u_MaterialParams; // x = metallic, y = roughness, z = ao, w = exposure
	GLint u_NumLights;
	GLint u_LightPosRadius[SLANG_MAX_LIGHTS];
	GLint u_LightColorIntensity[SLANG_MAX_LIGHTS];

	GLint u_AlbedoMap;
	GLint u_NormalMap;
	GLint u_RoughnessMap;
	GLint u_LightMap;
} slang_pbr_uniforms_t;

typedef struct slang_ssao_uniforms_s
{
	GLint u_Projection;
	GLint u_InvProjection;
	GLint u_View;
	GLint u_ScreenParams; // x = width, y = height, z = near, w = far
	GLint u_SSAOParams;   // x = radius, y = bias, z = intensity, w = power
	GLint u_Samples;

	GLint u_DepthMap;
	GLint u_NormalMap;
	GLint u_NoiseMap;
} slang_ssao_uniforms_t;

typedef struct slang_fxaa_uniforms_s
{
	GLint u_ScreenResolution; // xy = (w, h), zw = (1/w, 1/h)
	GLint u_ColorMap;
} slang_fxaa_uniforms_t;

typedef struct slang_pipeline_s
{
	qboolean initialized;

	// Shader program handles
	GLuint pbr_program;
	slang_pbr_uniforms_t pbr_uniforms;

	GLuint ssao_program;
	slang_ssao_uniforms_t ssao_uniforms;

	GLuint fxaa_program;
	slang_fxaa_uniforms_t fxaa_uniforms;
} slang_pipeline_t;

extern slang_pipeline_t g_slang_pipeline;

// API
qboolean GL_Slang_Init( void );
void     GL_Slang_Shutdown( void );
GLuint   GL_Slang_CompileProgram( const char *name, const char *vert_src, const char *frag_src );
void     GL_Slang_UseProgram( GLuint prog );

#endif // GL_SLANG_H
