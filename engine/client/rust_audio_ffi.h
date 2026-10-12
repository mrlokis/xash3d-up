/*
* Xash3D Rust Audio Engine C FFI header
*/

#ifndef RUST_AUDIO_FFI_H
#define RUST_AUDIO_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int xash_rust_audio_init( void );
void xash_rust_audio_shutdown( void );

void xash_rust_audio_update_listener(
	const float origin[3],
	const float forward[3],
	const float right[3],
	const float up[3]
);

int xash_rust_audio_play(
	const char *name,
	int entnum,
	int channel,
	const float origin[3],
	float volume,
	float attenuation,
	float pitch
);

void xash_rust_audio_stop( int entnum, int channel );
void xash_rust_audio_stop_all( void );
void xash_rust_audio_set_volume( float master_volume, float music_volume );
int xash_rust_audio_mix( int16_t *out_samples, uint32_t num_samples );

#ifdef __cplusplus
}
#endif

#endif // RUST_AUDIO_FFI_H
