# -*- mode: python ; coding: utf-8 -*-
import os
import ortools

ortools_path = os.path.dirname(ortools.__file__)
ortools_binaries = []

def should_exclude_collected_binary(binary):
    destination, source, typecode = binary
    if typecode != 'BINARY':
        return False

    binary_name = os.path.basename(destination).lower()
    if binary_name == 'msvcp140.dll':
        return True

    if binary_name == 'ucrtbase.dll':
        return True

    return binary_name.startswith('api-ms-win-')

pywrap_path = os.path.join(ortools_path, '_pywrapcp.pyd')
if os.path.exists(pywrap_path):
    ortools_binaries.append((pywrap_path, 'ortools'))
    
libs_path = os.path.join(ortools_path, '.libs')
if os.path.exists(libs_path):
    ortools_binaries.append((libs_path, 'ortools/.libs'))
    

a = Analysis(
    ['solver.py'],
    pathex=[],
    binaries=ortools_binaries,
    datas=[],
    hiddenimports=[],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[],
    noarchive=False,
    optimize=0,
)
a.binaries = [binary for binary in a.binaries if not should_exclude_collected_binary(binary)]
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name='solver',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)