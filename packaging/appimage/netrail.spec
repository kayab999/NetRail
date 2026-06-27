# -*- mode: python ; coding: utf-8 -*-

from pathlib import Path

root = Path(SPECPATH).resolve().parent.parent

a = Analysis(
    [str(root / "netrail" / "__main__.py")],
    pathex=[str(root)],
    binaries=[],
    datas=[(str(root / "netrail" / "static"), "netrail/static")],
    hiddenimports=["uvicorn.logging", "uvicorn.loops", "uvicorn.loops.auto", "uvicorn.protocols", "uvicorn.protocols.http", "uvicorn.protocols.http.auto", "uvicorn.lifespan", "uvicorn.lifespan.on"],
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
    name="netrail",
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