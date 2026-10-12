/*
gl_pbr.c - Physically Based Rendering, Realtime Dynamic Lights, SSAO, and FXAA pipeline
Copyright (C) 2026 Xash3D FWGS contributors
*/

#include "gl_local.h"
#include "gl_pbr.h"
#include "gl_slang.h"
#include <math.h>

gl_pbr_state_t g_pbr_state;

static slang_light_t s_pbr_lights[SLANG_MAX_LIGHTS];
static int           s_num_pbr_lights = 0;
static GLuint        s_screenColorTex = 0;
static int           s_screenTexW = 0;
static int           s_screenTexH = 0;

static float RandFloat( float min_val, float max_val )
{
	return min_val + (float)rand() / (float)RAND_MAX * (max_val - min_val);
}

void GL_PBR_Init( void )
{
	if( g_pbr_state.initialized )
		return;

	memset( &g_pbr_state, 0, sizeof( g_pbr_state ));

	// Initialize Slang shader pipeline
	GL_Slang_Init();

	// 1. Generate SSAO 16-sample hemisphere kernel
	for( int i = 0; i < 16; i++ )
	{
		vec3_t sample;
		sample[0] = RandFloat( -1.0f, 1.0f );
		sample[1] = RandFloat( -1.0f, 1.0f );
		sample[2] = RandFloat( 0.1f, 1.0f ); // hemisphere
		VectorNormalize( sample );

		float scale = (float)i / 16.0f;
		scale = 0.1f + scale * scale * (1.0f - 0.1f); // lerp
		sample[0] *= scale;
		sample[1] *= scale;
		sample[2] *= scale;

		g_pbr_state.ssao_kernel[i][0] = sample[0];
		g_pbr_state.ssao_kernel[i][1] = sample[1];
		g_pbr_state.ssao_kernel[i][2] = sample[2];
		g_pbr_state.ssao_kernel[i][3] = 0.0f;
	}

	// 2. Generate 4x4 random noise rotation texture for SSAO
	vec4_t noise_data[16];
	for( int i = 0; i < 16; i++ )
	{
		noise_data[i][0] = RandFloat( -1.0f, 1.0f ) * 0.5f + 0.5f;
		noise_data[i][1] = RandFloat( -1.0f, 1.0f ) * 0.5f + 0.5f;
		noise_data[i][2] = 0.5f;
		noise_data[i][3] = 1.0f;
	}

	byte noise_bytes[16 * 4];
	for( int i = 0; i < 16; i++ )
	{
		noise_bytes[i * 4 + 0] = (byte)(noise_data[i][0] * 255.0f);
		noise_bytes[i * 4 + 1] = (byte)(noise_data[i][1] * 255.0f);
		noise_bytes[i * 4 + 2] = (byte)(noise_data[i][2] * 255.0f);
		noise_bytes[i * 4 + 3] = 255;
	}

	g_pbr_state.noise_texture = (GLuint)GL_CreateTexture( "*ssao_noise", 4, 4, noise_bytes, TF_NOMIPMAP | TF_CLAMP | TF_HAS_ALPHA );

	g_pbr_state.initialized = true;
	gEngfuncs.Con_Printf( "^2[PBR] Modern Physically Based Rendering Pipeline active^7\n" );
}

void GL_PBR_Shutdown( void )
{
	if( !g_pbr_state.initialized )
		return;

	GL_Slang_Shutdown();

	if( g_pbr_state.ssao_fbo && pglDeleteFramebuffers )
		pglDeleteFramebuffers( 1, &g_pbr_state.ssao_fbo );

	if( g_pbr_state.ssao_tex )
		GL_FreeTexture( g_pbr_state.ssao_tex );

	if( g_pbr_state.noise_texture )
		GL_FreeTexture( g_pbr_state.noise_texture );

	if( s_screenColorTex && pglDeleteTextures )
	{
		pglDeleteTextures( 1, &s_screenColorTex );
		s_screenColorTex = 0;
		s_screenTexW = 0;
		s_screenTexH = 0;
	}

	memset( &g_pbr_state, 0, sizeof( g_pbr_state ));
}

void GL_PBR_UpdateDynamicLights( void )
{
	s_num_pbr_lights = 0;

	if( !r_realtime_lights.value || !tr.elights )
		return;

	for( int i = 0; i < MAX_DLIGHTS && s_num_pbr_lights < SLANG_MAX_LIGHTS; i++ )
	{
		dlight_t *dl = &tr.elights[i];
		if( dl->die < gp_cl->time || dl->radius <= 0.0f )
			continue;

		s_pbr_lights[s_num_pbr_lights].position_radius[0] = dl->origin[0];
		s_pbr_lights[s_num_pbr_lights].position_radius[1] = dl->origin[1];
		s_pbr_lights[s_num_pbr_lights].position_radius[2] = dl->origin[2];
		s_pbr_lights[s_num_pbr_lights].position_radius[3] = dl->radius * 2.5f;

		s_pbr_lights[s_num_pbr_lights].color_intensity[0] = (float)dl->color.r / 255.0f;
		s_pbr_lights[s_num_pbr_lights].color_intensity[1] = (float)dl->color.g / 255.0f;
		s_pbr_lights[s_num_pbr_lights].color_intensity[2] = (float)dl->color.b / 255.0f;
		s_pbr_lights[s_num_pbr_lights].color_intensity[3] = 2.5f;

		s_num_pbr_lights++;
	}
}

void GL_PBR_Begin( const matrix4x4 mvp, const matrix4x4 model, const vec3_t viewpos )
{
	(void)mvp; (void)model; (void)viewpos;
}

void GL_PBR_End( void )
{
}

static void GL_DrawFullscreenQuad( int width, int height )
{
	matrix4x4 m;
	GLboolean depth_test = pglIsEnabled ? pglIsEnabled( GL_DEPTH_TEST ) : GL_TRUE;
	GLboolean blend = pglIsEnabled ? pglIsEnabled( GL_BLEND ) : GL_FALSE;

	pglViewport( 0, 0, width, height );

	pglMatrixMode( GL_PROJECTION );
	pglPushMatrix();
	Matrix4x4_CreateOrtho( m, 0, (float)width, (float)height, 0.0f, -1.0f, 1.0f );
	GL_LoadMatrix( m );

	pglMatrixMode( GL_MODELVIEW );
	pglPushMatrix();
	Matrix4x4_LoadIdentity( m );
	GL_LoadMatrix( m );

	GL_Cull( GL_NONE );
	pglDisable( GL_DEPTH_TEST );
	pglDepthMask( GL_FALSE );
	pglDisable( GL_BLEND );
	pglDisable( GL_ALPHA_TEST );
	pglColor4f( 1.0f, 1.0f, 1.0f, 1.0f );

	pglBegin( GL_QUADS );
		pglTexCoord2f( 0.0f, 1.0f );
		pglVertex2f( 0.0f, 0.0f );
		pglTexCoord2f( 1.0f, 1.0f );
		pglVertex2f( (float)width, 0.0f );
		pglTexCoord2f( 1.0f, 0.0f );
		pglVertex2f( (float)width, (float)height );
		pglTexCoord2f( 0.0f, 0.0f );
		pglVertex2f( 0.0f, (float)height );
	pglEnd();

	pglMatrixMode( GL_MODELVIEW );
	pglPopMatrix();
	pglMatrixMode( GL_PROJECTION );
	pglPopMatrix();

	if( depth_test ) pglEnable( GL_DEPTH_TEST );
	if( blend ) pglEnable( GL_BLEND );
}

void GL_PBR_ApplySSAO( GLuint depth_tex, int width, int height, const matrix4x4 proj )
{
	if( !r_ssao.value || !g_slang_pipeline.ssao_program || !depth_tex )
		return;

	// Create or resize SSAO FBO if needed
	if( !g_pbr_state.ssao_fbo || g_pbr_state.ssao_width != width || g_pbr_state.ssao_height != height )
	{
		if( g_pbr_state.ssao_fbo && pglDeleteFramebuffers )
			pglDeleteFramebuffers( 1, &g_pbr_state.ssao_fbo );
		if( g_pbr_state.ssao_tex )
			GL_FreeTexture( g_pbr_state.ssao_tex );

		g_pbr_state.ssao_tex = (GLuint)GL_CreateTexture( "*ssao_buffer", width, height, NULL, TF_NOMIPMAP | TF_CLAMP );
		if( pglGenFramebuffers && pglBindFramebuffer )
		{
			pglGenFramebuffers( 1, &g_pbr_state.ssao_fbo );
			pglBindFramebuffer( GL_FRAMEBUFFER, g_pbr_state.ssao_fbo );
			gl_texture_t *tex = R_GetTexture( g_pbr_state.ssao_tex );
			if( tex && pglFramebufferTexture2D )
				pglFramebufferTexture2D( GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, tex->texnum, 0 );
			pglBindFramebuffer( GL_FRAMEBUFFER, 0 );
		}

		g_pbr_state.ssao_width = width;
		g_pbr_state.ssao_height = height;
	}

	if( !g_pbr_state.ssao_fbo )
		return;

	// Render SSAO pass
	pglBindFramebuffer( GL_FRAMEBUFFER, g_pbr_state.ssao_fbo );
	GL_Slang_UseProgram( g_slang_pipeline.ssao_program );

	slang_ssao_uniforms_t *u = &g_slang_pipeline.ssao_uniforms;
	if( u->u_Projection >= 0 && pglUniformMatrix4fvARB )
		pglUniformMatrix4fvARB( u->u_Projection, 1, GL_FALSE, (const GLfloat *)proj );

	if( u->u_ScreenParams >= 0 && pglUniform4fARB )
		pglUniform4fARB( u->u_ScreenParams, (float)width, (float)height, 4.0f, 4096.0f );

	if( u->u_SSAOParams >= 0 && pglUniform4fARB )
		pglUniform4fARB( u->u_SSAOParams, 0.65f, 0.025f, 1.2f, 1.5f ); // radius, bias, intensity, power

	if( u->u_Samples >= 0 && pglUniform4fvARB )
		pglUniform4fvARB( u->u_Samples, 16, (const GLfloat *)g_pbr_state.ssao_kernel );

	// Bind textures: Depth (TMU 0), Noise (TMU 2)
	GL_SelectTexture( 0 );
	pglBindTexture( GL_TEXTURE_2D, depth_tex );
	glState.currentTextures[0] = depth_tex;
	if( u->u_DepthMap >= 0 && pglUniform1iARB ) pglUniform1iARB( u->u_DepthMap, 0 );

	GL_Bind( XASH_TEXTURE2, g_pbr_state.noise_texture );
	if( u->u_NoiseMap >= 0 && pglUniform1iARB ) pglUniform1iARB( u->u_NoiseMap, 2 );

	GL_DrawFullscreenQuad( width, height );

	GL_Slang_UseProgram( 0 );
	pglBindFramebuffer( GL_FRAMEBUFFER, 0 );
}

void GL_PBR_ApplyFXAA( GLuint scene_tex, int width, int height )
{
	if( !r_fxaa.value || !g_slang_pipeline.fxaa_program || !scene_tex )
		return;

	GL_Slang_UseProgram( g_slang_pipeline.fxaa_program );

	slang_fxaa_uniforms_t *u = &g_slang_pipeline.fxaa_uniforms;
	if( u->u_ScreenResolution >= 0 && pglUniform4fARB )
		pglUniform4fARB( u->u_ScreenResolution, (float)width, (float)height, 1.0f / (float)width, 1.0f / (float)height );

	GL_SelectTexture( 0 );
	pglBindTexture( GL_TEXTURE_2D, scene_tex );
	glState.currentTextures[0] = scene_tex;
	if( u->u_ColorMap >= 0 && pglUniform1iARB ) pglUniform1iARB( u->u_ColorMap, 0 );

	GL_DrawFullscreenQuad( width, height );

	GL_Slang_UseProgram( 0 );
}

static GLuint GL_PBR_GetScreenTexture( int width, int height )
{
	if( !s_screenColorTex || s_screenTexW != width || s_screenTexH != height )
	{
		if( s_screenColorTex )
			pglDeleteTextures( 1, &s_screenColorTex );
		pglGenTextures( 1, &s_screenColorTex );
		GL_SelectTexture( 0 );
		pglBindTexture( GL_TEXTURE_2D, s_screenColorTex );
		pglTexParameteri( GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR );
		pglTexParameteri( GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR );
		pglTexParameteri( GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE );
		pglTexParameteri( GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE );
		pglTexImage2D( GL_TEXTURE_2D, 0, GL_RGBA8, width, height, 0, GL_RGBA, GL_UNSIGNED_BYTE, NULL );
		s_screenTexW = width;
		s_screenTexH = height;
	}
	return s_screenColorTex;
}

void GL_PBR_PostProcess( int width, int height, const vec3_t viewpos )
{
	if( !r_pbr.value && !r_fxaa.value )
		return;

	if( width <= 0 || height <= 0 )
		return;

	GLuint screenTex = GL_PBR_GetScreenTexture( width, height );
	GL_SelectTexture( 0 );
	pglBindTexture( GL_TEXTURE_2D, screenTex );
	glState.currentTextures[0] = screenTex;
	pglCopyTexSubImage2D( GL_TEXTURE_2D, 0, 0, 0, 0, 0, width, height );

	// 1. PBR Post-process & Realtime dynamic lighting pass
	if( r_pbr.value && g_slang_pipeline.pbr_program )
	{
		GL_Slang_UseProgram( g_slang_pipeline.pbr_program );
		slang_pbr_uniforms_t *u = &g_slang_pipeline.pbr_uniforms;

		if( u->u_ViewPosition >= 0 && pglUniform4fARB )
			pglUniform4fARB( u->u_ViewPosition, viewpos[0], viewpos[1], viewpos[2], 1.0f );

		if( u->u_SunDirection_Intensity >= 0 && pglUniform4fARB )
			pglUniform4fARB( u->u_SunDirection_Intensity, 0.45f, -0.6f, -0.65f, 1.8f );

		if( u->u_SunColor >= 0 && pglUniform4fARB )
			pglUniform4fARB( u->u_SunColor, 1.0f, 0.95f, 0.85f, 1.0f );

		float amb = bound( 0.05f, r_lighting_ambient.value, 1.0f );
		if( u->u_AmbientColor >= 0 && pglUniform4fARB )
			pglUniform4fARB( u->u_AmbientColor, amb, amb, amb, 1.0f );

		if( u->u_MaterialParams >= 0 && pglUniform4fARB )
			pglUniform4fARB( u->u_MaterialParams, 0.08f, 0.45f, 1.0f, 1.0f );

		if( r_realtime_lights.value )
			GL_PBR_UpdateDynamicLights();
		else
			s_num_pbr_lights = 0;

		if( u->u_NumLights >= 0 && pglUniform1iARB )
			pglUniform1iARB( u->u_NumLights, s_num_pbr_lights );

		for( int i = 0; i < s_num_pbr_lights; i++ )
		{
			if( u->u_LightPosRadius[i] >= 0 && pglUniform4fvARB )
				pglUniform4fvARB( u->u_LightPosRadius[i], 1, s_pbr_lights[i].position_radius );
			if( u->u_LightColorIntensity[i] >= 0 && pglUniform4fvARB )
				pglUniform4fvARB( u->u_LightColorIntensity[i], 1, s_pbr_lights[i].color_intensity );
		}

		GL_SelectTexture( 0 );
		pglBindTexture( GL_TEXTURE_2D, screenTex );
		glState.currentTextures[0] = screenTex;
		if( u->u_AlbedoMap >= 0 && pglUniform1iARB )
			pglUniform1iARB( u->u_AlbedoMap, 0 );

		GL_DrawFullscreenQuad( width, height );
		GL_Slang_UseProgram( 0 );

		if( r_fxaa.value )
		{
			GL_SelectTexture( 0 );
			pglBindTexture( GL_TEXTURE_2D, screenTex );
			glState.currentTextures[0] = screenTex;
			pglCopyTexSubImage2D( GL_TEXTURE_2D, 0, 0, 0, 0, 0, width, height );
		}
	}

	// 2. FXAA Anti-aliasing pass
	if( r_fxaa.value && g_slang_pipeline.fxaa_program )
	{
		GL_PBR_ApplyFXAA( screenTex, width, height );
	}
}
