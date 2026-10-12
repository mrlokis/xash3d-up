/*
* Xash3D Jolt Physics FFI interface header
*/

#ifndef RUST_JOLT_FFI_H
#define RUST_JOLT_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct jolt_plane_s
{
	float normal[3];
	float dist;
	uint8_t type;
	uint8_t signbits;
	uint8_t pad[2];
} jolt_plane_t;

typedef struct jolt_brush_s
{
	uint32_t first_plane;
	uint32_t num_planes;
	float mins[3];
	float maxs[3];
	int32_t contents;
} jolt_brush_t;

typedef struct jolt_trace_result_s
{
	int32_t allsolid;
	int32_t startsolid;
	int32_t inopen;
	int32_t inwater;
	float fraction;
	float endpos[3];
	float plane_normal[3];
	float plane_dist;
	int32_t hit_ent;
} jolt_trace_result_t;

int xash_rust_jolt_init( void );
void xash_rust_jolt_shutdown( void );
int xash_rust_jolt_level_init( const jolt_brush_t *brushes, uint32_t num_brushes, const jolt_plane_t *planes, uint32_t num_planes );
int xash_rust_jolt_level_init_mesh( const float *vertices, uint32_t num_vertices, const uint32_t *indices, uint32_t num_triangles );
void xash_rust_jolt_level_free( void );
int xash_rust_jolt_step( float dt );

int xash_rust_jolt_character_update(
	int player_index,
	float origin[3],
	float velocity[3],
	const float angles[3],
	int buttons,
	float forwardmove,
	float sidemove,
	float upmove,
	float maxspeed,
	int *flags,
	float frametime,
	float gravity
);
void xash_rust_jolt_character_remove( int player_index );

int xash_rust_jolt_entity_create(
	int ent_index,
	int shape_type,
	const float mins[3],
	const float maxs[3],
	const float origin[3],
	const float velocity[3],
	float mass
);
void xash_rust_jolt_entity_destroy( int ent_index );
int xash_rust_jolt_entity_get_transform( int ent_index, float origin[3], float angles[3] );
int xash_rust_jolt_entity_set_transform( int ent_index, const float origin[3], const float angles[3] );
int xash_rust_jolt_entity_get_velocity( int ent_index, float velocity[3] );
int xash_rust_jolt_entity_set_velocity( int ent_index, const float velocity[3] );

int xash_rust_jolt_trace_line( const float start[3], const float end[3], jolt_trace_result_t *result );
int xash_rust_jolt_trace_hull( const float start[3], const float end[3], const float mins[3], const float maxs[3], jolt_trace_result_t *result );

#ifdef __cplusplus
}
#endif

#endif // RUST_JOLT_FFI_H
