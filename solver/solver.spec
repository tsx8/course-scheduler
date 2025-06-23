# -*- mode: python ; coding: utf-8 -*-
import os
import ortools

ortools_path = os.path.dirname(ortools.__file__)
ortools_binaries = []

pywrap_path = os.path.join(ortools_path, '_pywrapcp.pyd')
if os.path.exists(pywrap_path):
    ortools_binaries.append((pywrap_path, 'ortools'))
    
libs_path = os.path.join(ortools_path, '.libs')
if os.path.exists(libs_path):
    ortools_binaries.append((libs_path, 'ortools/.libs'))
    
print(ortools_binaries)

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
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)