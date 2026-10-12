/*
gl_pbr.h - Physically Based Rendering, Realtime Dynamic Lights, SSAO, and FXAA pipeline
Copyright (C) 2026 Xash3D FWGS contributors
*/

#ifndef GL_PBR_H
#define GL_PBR_H

#include "gl_local.h"
#include "gl_slang.h"

typedef struct
{
	qboolean initialized;
	GLuint noise_texture;
	vec4_t ssao_kernel[16];

	// SSAO FBO
	GLuint ssao_fbo;
	GLuint ssao_tex;
	int ssao_width;
	int ssao_height;

	// Postprocessing state
	qboolean pbr_active;
} gl_pbr_state_t;

extern gl_pbr_state_t g_pbr_state;

// PBR Pipeline API
void GL_PBR_Init( void );
void GL_PBR_Shutdown( void );
void GL_PBR_UpdateDynamicLights( void );
void GL_PBR_Begin( const matrix4x4 mvp, const matrix4x4 model, const vec3_t viewpos );
void GL_PBR_End( void );
void GL_PBR_ApplySSAO( GLuint depth_tex, int width, int height, const matrix4x4 proj );
void GL_PBR_ApplyFXAA( GLuint scene_tex, int width, int height );
void GL_PBR_PostProcess( int width, int height, const vec3_t viewpos );

#endif // GL_PBR_H
