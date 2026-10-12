# encoding: utf-8
# xcompile.py -- crosscompiling utils
# Copyright (C) 2018 a1batross
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.

try: from fwgslib import get_flags_by_compiler
except: from waflib.extras.fwgslib import get_flags_by_compiler
from waflib import Logs, TaskGen
from waflib.Tools import c_config
from collections import OrderedDict
import os
import sys

NSWITCH_ENVVARS = ['DEVKITPRO']

PSVITA_ENVVARS = ['VITASDK']

PSP_ENVVARS = ['PSPDEV', 'PSPSDK', 'PSPTOOLCHAIN']

class iOS:
	ctx = None
	sdkpath = None
	target = None
	
	def __init__(self, ctx, issim):
		self.ctx = ctx
		sdk = 'iphonesimulator' if issim else 'iphoneos'
		self.sdkpath = ctx.cmd_and_log('xcrun --show-sdk-path --sdk %s' % sdk).strip()
		self.target = '--target=aarch64-apple-ios'
		if issim: self.target += '-simulator'
	
	def cflags(self, cxx = False):
	
		cflags = [ '-isysroot' + self.sdkpath, self.target, '-mios-version-min=12.0' ]
		return cflags
		
	def linkflags(self):
		
		linkflags = [ '-isysroot' + self.sdkpath, self.target, '-mios-version-min=12.0' ]
		return linkflags
	
	def cc(self):
		return 'clang'

	def cxx(self):
		return 'clang++'

class NintendoSwitch:
	ctx          = None # waf context
	arch         = "arm64"
	dkp_dir      = None
	portlibs_dir = None
	dka64_dir    = None
	libnx_dir    = None

	def __init__(self, ctx):
		self.ctx = ctx

		for i in NSWITCH_ENVVARS:
			self.dkp_dir = os.getenv(i)
			if self.dkp_dir != None:
				break
		else:
			ctx.fatal('Set %s environment variable pointing to the DEVKITPRO home!' %
				' or '.join(NSWITCH_ENVVARS))

		self.dkp_dir = os.path.abspath(self.dkp_dir)

		self.dka64_dir = os.path.join(self.dkp_dir, 'devkitA64')
		if not os.path.exists(self.dka64_dir):
			ctx.fatal('devkitA64 not found in `%s`. Install devkitA64!' % self.dka64_dir)

		self.libnx_dir = os.path.join(self.dkp_dir, 'libnx')
		if not os.path.exists(self.libnx_dir):
			ctx.fatal('libnx not found in `%s`. Install libnx!' % self.libnx_dir)

		self.portlibs_dir = os.path.join(self.dkp_dir, 'portlibs', 'switch')
		if not os.path.exists(self.portlibs_dir):
			ctx.fatal('No Switch libraries found in `%s`!' % self.portlibs_dir)

	def gen_toolchain_prefix(self):
		return 'aarch64-none-elf-'

	def gen_gcc_toolchain_path(self):
		return os.path.join(self.dka64_dir, 'bin', self.gen_toolchain_prefix())

	def cc(self):
		return self.gen_gcc_toolchain_path() + 'gcc'

	def cxx(self):
		return self.gen_gcc_toolchain_path() + 'g++'

	def strip(self):
		return self.gen_gcc_toolchain_path() + 'strip'

	def pkgconfig(self):
		# counter-intuitively, this motherfucker is in $DEVKITPRO/portlibs/switch/bin
		return os.path.join(self.portlibs_dir, 'bin', self.gen_toolchain_prefix() + 'pkg-config')

	def cflags(self, cxx = False):
		cflags = []
		# arch flags
		cflags += ['-D__SWITCH__', '-march=armv8-a+crc+crypto', '-mtune=cortex-a57', '-mtp=soft', '-ftls-model=local-exec', '-fPIE']
		# help the linker out
		cflags += ['-ffunction-sections', '-fdata-sections']
		# base include dirs
		# portlibs must be -isystem too, so bundled libraries headers (e.g. mpg123) take precedence over portlibs ones
		cflags += ['-isystem %s/include' % self.libnx_dir, '-isystem %s/include' % self.portlibs_dir]
		# the game wants GNU extensions
		if cxx:
			cflags += ['-std=gnu++17', '-D_GNU_SOURCE']
		else:
			cflags += ['-std=gnu11', '-D_GNU_SOURCE']
		return cflags

	# they go before object list
	def linkflags(self):
		linkflags = ['-fPIE', '-specs=%s/switch.specs' % self.libnx_dir]
		# libsolder only supports sysv hashes and we need to build everything with -rdynamic
		linkflags += ['-Wl,--hash-style=sysv', '-rdynamic']
		# avoid pulling in and exposing mesa's internals, that crashes it for some god forsaken reason
		linkflags += ['-Wl,--exclude-libs=libglapi.a', '-Wl,--exclude-libs=libdrm_nouveau.a']
		return linkflags

	def ldflags(self):
		# NOTE: shared libraries should be built without standard libs, so that they could import their contents from the NRO,
		# but executables, including the SDL2 sanity check, will generally require libstdc++ and libm, which we will add manually
		ldflags = [] # ['-lm', '-lstdc++']
		return ldflags

class PSVita:
	ctx          = None # waf context
	arch         ='armeabi-v7a-hard'
	vitasdk_dir  = None

	def __init__(self, ctx):
		self.ctx = ctx

		for i in PSVITA_ENVVARS:
			self.vitasdk_dir = os.getenv(i)
			if self.vitasdk_dir != None:
				break
		else:
			ctx.fatal('Set %s environment variable pointing to the VitaSDK directory!' %
				' or '.join(PSVITA_ENVVARS))

	def gen_toolchain_prefix(self):
		return 'arm-vita-eabi-'

	def gen_gcc_toolchain_path(self):
		return os.path.join(self.vitasdk_dir, 'bin', self.gen_toolchain_prefix())

	def cc(self):
		return self.gen_gcc_toolchain_path() + 'gcc'

	def cxx(self):
		return self.gen_gcc_toolchain_path() + 'g++'

	def strip(self):
		return self.gen_gcc_toolchain_path() + 'strip'

	def ar(self):
		return self.gen_gcc_toolchain_path() + 'ar'

	def pkgconfig(self):
		return self.gen_gcc_toolchain_path() + 'pkg-config'

	def cflags(self, cxx = False):
		cflags = []
		# arch flags
		cflags += ['-D__vita__', '-mtune=cortex-a9', '-mfpu=neon']
		# necessary linker flags
		cflags += ['-Wl,-q', '-Wl,-z,nocopyreloc']
		# this optimization is broken in vitasdk
		cflags += ['-fno-optimize-sibling-calls']
		# disable some ARM bullshit
		cflags += ['-fno-short-enums', '-Wno-attributes']
		# base include dir
		cflags += ['-isystem %s/arm-vita-eabi/include' % self.vitasdk_dir]
		# SDL include dir
		cflags += ['-I%s/arm-vita-eabi/include/SDL2' % self.vitasdk_dir]
		return cflags

	# they go before object list
	def linkflags(self):
		linkflags = ['-Wl,--hash-style=sysv', '-Wl,-q', '-Wl,-z,nocopyreloc', '-mtune=cortex-a9', '-mfpu=neon']
		# enforce no-short-enums again
		linkflags += ['-Wl,-no-enum-size-warning', '-fno-short-enums']
		# vitasdk's ld script reserves __sce_headroom bytes after the RX segment for the SCE metadata that vita-elf-create appends there,
		# otherwise "Cannot allocate N bytes for SCE data at end of segment 0; segment 1 overlaps" depends on the layout, see vitasdk/buildscripts#144
		linkflags += ['-Wl,-z,max-page-size=0x10000', '-Wl,--defsym,__sce_headroom=0x10000']
		return linkflags

	def ldflags(self):
		ldflags = []
		return ldflags

class PSP:
	ctx               = None # waf context
	sdk_home          = None
	psptoolchain_path = None
	pspsdk_path       = None
	binutils_path     = None
	build_prx         = None
	fw_version        = None
	render_type       = None

	def __init__(self, ctx, moduletype, fwversion, rendertype):
		self.ctx = ctx
		self.build_prx = True if moduletype == 'prx' else False
		self.fw_version = fwversion
		self.render_type = rendertype

		for i in PSP_SDK_ENVVARS:
			self.sdk_home = os.getenv(i)
			if self.sdk_home != None:
				break
		else:
			ctx.fatal('Set %s environment variable pointing to the root of PSP SDK!' %
				' or '.join(PSP_SDK_ENVVARS))

		self.psptoolchain_path = os.path.join(self.sdk_home, 'psp')
		self.pspsdk_path = os.path.join(self.psptoolchain_path, 'sdk')
		self.binutils_path  = os.path.join(self.sdk_home, 'bin')

	def cflags(self, cxx = False):
		cflags = []
		cflags += ['-I%s' % (os.path.join(self.pspsdk_path, 'include'))]
		cflags += ['-I.']
		cflags += ['-DNDEBUG', '-D_PSP_FW_VERSION=%s' % self.fw_version, '-G0']
		return cflags

	# they go before object list
	def linkflags(self):
		linkflags = []
		return linkflags

	def ldflags(self):
		ldflags = []
		if self.build_prx:
			ldflags += ['-specs=%s' % os.path.join(self.pspsdk_path, 'lib/prxspecs')]
			ldflags += ['-Wl,-q,-T%s' % os.path.join(self.pspsdk_path, 'lib/linkfile.prx')]
		ldflags += ['-L%s' % os.path.join(self.pspsdk_path, 'lib')]
		ldflags += ['-L.']
		return ldflags

	def stdlibs(self):
		stdlibs = []
		stdlibs += ['-lpspdisplay', '-lpspgum_vfpu', '-lpspgu','-lpspge', '-lpspvfpu']
		stdlibs += ['-lpspaudio', '-lpspdmac']
		stdlibs += ['-lstdc++', '-lc', '-lm']
		stdlibs += ['-lpspctrl', '-lpspdebug', '-lpsppower', '-lpsputility',  '-lpspsdk', '-lpsprtc']
		stdlibs += ['-lpspuser', '-lpspkernel']
		return stdlibs

def options(opt):
	xc = opt.add_option_group('Cross compile options')
	xc.add_option('--enable-magx', action='store_true', dest='MAGX', default=False,
		help='enable building for Motorola MAGX [default: %(default)s]')
	xc.add_option('--enable-msvc-wine', action='store_true', dest='MSVC_WINE', default=False,
		help='enable building with MSVC using Wine [default: %(default)s]')
	xc.add_option('--nswitch', action='store_true', dest='NSWITCH', default = False,
		help='enable building for Nintendo Switch [default: %(default)s]')
	xc.add_option('--psvita', action='store_true', dest='PSVITA', default = False,
		help='enable building for PlayStation Vita [default: %(default)s]')
	xc.add_option('--sailfish', action='store_true', dest='SAILFISH', default = False,
		help='enable building for Sailfish')
	xc.add_option('--emscripten', action='store_true', dest='EMSCRIPTEN', default = None,
		help='enable building for Emscripten')
	xc.add_option('--psp', action='store', dest='PSP_OPTS', default=None,
		help='enable building for PlayStation Portable, format: --psp=<module type>,<fw version>,<render type>, example: --psp=prx,660,HW')
	xc.add_option('--ios', action='store_true', dest='IOS', default=False, 
		help='enable building for iOS [default: %(default)s]')
	xc.add_option('--ios-simulator', action='store_true', dest='IOSSIM', default=False, 
		help='enable building for iOS Simulator [default: %(default)s]')

def configure(conf):
	if 'CROSS_COMPILE' in conf.environ:
		toolchain_path = conf.environ['CROSS_COMPILE']
		conf.environ['CC'] = toolchain_path + 'cc'
		conf.environ['CXX'] = toolchain_path + 'c++'
		conf.environ['STRIP'] = toolchain_path + 'strip'
		conf.environ['OBJDUMP'] = toolchain_path + 'objdump'
		conf.environ['AR'] = toolchain_path + 'ar'
	elif conf.options.MAGX:
		# useless to change toolchain path, as toolchain meant to be placed in this path
		toolchain_path = '/opt/toolchains/motomagx/arm-eabi2/lib/'
		conf.env.INCLUDES_MAGX = [toolchain_path + i for i in ['ezx-z6/include', 'qt-2.3.8/include']]
		conf.env.LIBPATH_MAGX  = [toolchain_path + i for i in ['ezx-z6/lib', 'qt-2.3.8/lib']]
		conf.env.LINKFLAGS_MAGX = ['-Wl,-rpath-link=' + i for i in conf.env.LIBPATH_MAGX]
	elif conf.options.MSVC_WINE:
		try:
			toolchain_path = conf.environ['MSVC_WINE_PATH']
		except KeyError:
			conf.fatal('Set MSVC_WINE_PATH environment variable to the MSVC toolchain root!')

		conf.environ['CC'] = conf.environ['CXX'] = os.path.join(toolchain_path, 'bin', conf.env.MSVC_TARGETS[0], 'cl')
		conf.environ['LINK_CXX'] = os.path.join(toolchain_path, 'bin', conf.env.MSVC_TARGETS[0], 'link')
		conf.environ['AR'] = os.path.join(toolchain_path, 'bin', conf.env.MSVC_TARGETS[0], 'lib')
		conf.environ['WINRC'] = os.path.join(toolchain_path, 'bin', conf.env.MSVC_TARGETS[0], 'rc')
		conf.env.DEST_OS = 'win32'
		conf.env.DEST_CPU = conf.env.MSVC_TARGETS[0]
		conf.env.COMPILER_CXX = conf.env.COMPILER_CC = 'msvc'
	elif conf.options.NSWITCH:
		conf.nswitch = nswitch = NintendoSwitch(conf)
		conf.environ['CC'] = nswitch.cc()
		conf.environ['CXX'] = nswitch.cxx()
		conf.environ['STRIP'] = nswitch.strip()
		conf.env.PKGCONFIG = nswitch.pkgconfig()
		conf.env.CFLAGS += nswitch.cflags()
		conf.env.CXXFLAGS += nswitch.cflags(True)
		conf.env.LINKFLAGS += nswitch.linkflags()
		conf.env.LDFLAGS += nswitch.ldflags()
		conf.env.HAVE_M = True
		conf.env.LIB_M = ['m']
		conf.env.DEST_OS = 'nswitch'
	elif conf.options.PSVITA:
		conf.psvita = psvita = PSVita(conf)
		conf.environ['CC'] = psvita.cc()
		conf.environ['CXX'] = psvita.cxx()
		conf.environ['STRIP'] = psvita.strip()
		conf.environ['AR'] = psvita.ar()
		conf.env.PKGCONFIG = psvita.pkgconfig()
		conf.env.CFLAGS += psvita.cflags()
		conf.env.CXXFLAGS += psvita.cflags(True)
		conf.env.LINKFLAGS += psvita.linkflags()
		conf.env.LDFLAGS += psvita.ldflags()
		conf.env.HAVE_M = True
		conf.env.LIB_M = ['m']
		conf.env.VRTLD = ['vrtld']
		conf.env.DEST_OS = 'psvita'
	elif conf.options.PSP_OPTS:
		values = conf.options.PSP_OPTS.split(',')
		if len(values) != 3:
			conf.fatal('Invalid --psp paramater value!')

		valid_module_type = ['elf', 'prx']
		valid_render_type = ['SW', 'HW', 'ALL']

		if values[0] not in valid_module_type:
			conf.fatal('Unknown module type: %s. Supported: %r' % (values[0], ', '.join(valid_module_type)))
		if values[2] not in valid_render_type:
			conf.fatal('Unknown render type: %s. Supported: %r' % (values[2], ', '.join(valid_render_type)))

		conf.psp = psp = PSP(conf, values[0], values[1], values[2])
		conf.environ['CC'] = os.path.join(psp.binutils_path, 'psp-gcc')
		conf.environ['CXX'] = os.path.join(psp.binutils_path, 'psp-gcc')
		conf.environ['AS'] = os.path.join(psp.binutils_path, 'psp-gcc')
		conf.environ['STRIP'] = os.path.join(psp.binutils_path, 'psp-strip')
		conf.environ['LD'] = os.path.join(psp.binutils_path, 'psp-gcc')
		conf.environ['AR'] = os.path.join(psp.binutils_path, 'psp-ar')
		conf.environ['RANLIB'] = os.path.join(psp.binutils_path, 'psp-ranlib')
		conf.environ['OBJCOPY'] = os.path.join(psp.binutils_path, 'psp-objcopy')

		conf.env.PRXGEN = conf.find_program('psp-prxgen', path_list = psp.binutils_path)
		conf.env.MKSFO =  conf.find_program('mksfoex', path_list = psp.binutils_path)
		conf.env.PACK_PBP = conf.find_program('pack-pbp', path_list = psp.binutils_path)
		conf.env.FIXUP =  conf.find_program('psp-fixup-imports', path_list = psp.binutils_path)

		conf.env.ASFLAGS += psp.cflags()
		conf.env.CFLAGS += psp.cflags()
		conf.env.CXXFLAGS += psp.cflags()
		conf.env.LINKFLAGS += psp.linkflags()
		conf.env.LDFLAGS += psp.ldflags()

		conf.msg('Selected PSPSDK', '%s' % (psp.sdk_home))
		# no need to print C/C++ compiler, as it would be printed by compiler_c/cxx
		conf.msg('... C/C++ flags', ' '.join(psp.cflags()).replace(psp.sdk_home, '$PSPSDK'))
		conf.msg('... link flags', ' '.join(psp.linkflags()).replace(psp.sdk_home, '$PSPSDK'))
		conf.msg('... ld flags', ' '.join(psp.ldflags()).replace(psp.sdk_home, '$PSPSDK'))
		conf.msg('Bulid prx', '%s' % (psp.build_prx))

		if conf.options.PROFILING:
			conf.env.LDFLAGS += ['-lpspprof']
			conf.env.CFLAGS += ['-pg']
			conf.env.CXXFLAGS += ['-pg']
		conf.env.LDFLAGS += psp.stdlibs()

		conf.env.PSP_RENDER_TYPE = psp.render_type
		conf.env.PSP_BUILD_PRX = psp.build_prx
	elif conf.options.IOS or conf.options.IOSSIM:
		issim = True if conf.options.IOSSIM else False

		conf.ios = ios = iOS(conf, issim)

		conf.environ['CC'] = ios.cc()
		conf.environ['CXX'] = ios.cxx()
		
		conf.env.CFLAGS += ios.cflags()
		conf.env.CXXFLAGS += ios.cflags()
		conf.env.LINKFLAGS += ios.linkflags()
		conf.env.IOS = 1

	conf.env.MAGX = conf.options.MAGX
	conf.env.MSVC_WINE = conf.options.MSVC_WINE
	conf.env.SAILFISH = conf.options.SAILFISH
	MACRO_TO_DESTOS = OrderedDict({ '__SWITCH__' : 'nswitch', '__vita__' : 'psvita', '__wasi__': 'wasi', '__EMSCRIPTEN__' : 'emscripten', '__psp__': 'psp' })
	for k in c_config.MACRO_TO_DESTOS:
		MACRO_TO_DESTOS[k] = c_config.MACRO_TO_DESTOS[k] # ordering is important
	c_config.MACRO_TO_DESTOS  = MACRO_TO_DESTOS

def post_compiler_cxx_configure(conf):
	conf.msg('Target OS', conf.env.DEST_OS)
	conf.msg('Target CPU', conf.env.DEST_CPU)
	conf.msg('Target binfmt', conf.env.DEST_BINFMT)

	if conf.options.MAGX:
		for lib in ['qte-mt', 'ezxappbase', 'ezxpm', 'log_util']:
			conf.check_cc(lib=lib, use='MAGX', uselib_store='MAGX')

	return

def post_compiler_c_configure(conf):
	conf.msg('Target OS', conf.env.DEST_OS)
	conf.msg('Target CPU', conf.env.DEST_CPU)
	conf.msg('Target binfmt', conf.env.DEST_BINFMT)

	return

from waflib.Tools import compiler_cxx, compiler_c

compiler_cxx_configure = getattr(compiler_cxx, 'configure')
compiler_c_configure = getattr(compiler_c, 'configure')

def patch_compiler_cxx_configure(conf):
	if not conf.env.MSVC_WINE:
		compiler_cxx_configure(conf)
	else:
		conf.load('msvc', funs='no_autodetect')
	post_compiler_cxx_configure(conf)

def patch_compiler_c_configure(conf):
	if not conf.env.MSVC_WINE:
		compiler_c_configure(conf)
	else:
		conf.load('msvc', funs='no_autodetect')
	post_compiler_c_configure(conf)

setattr(compiler_cxx, 'configure', patch_compiler_cxx_configure)
setattr(compiler_c, 'configure', patch_compiler_c_configure)

