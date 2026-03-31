# -*- mode: python ; coding: utf-8 -*-

from PyInstaller.utils.hooks import collect_data_files


hiddenimports = [
    "openai",
    "pynput",
    "pynput.mouse",
    "pynput.mouse._win32",
    "pycaw.pycaw",
    "pycaw.utils",
    "pycaw.constants",
    "comtypes.stream",
    "voice_input.llm_post_edit",
]

datas = []
datas += collect_data_files("pycaw")


a = Analysis(
    ["main.py"],
    pathex=[],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        "PyQt5",
        "PySide2",
        "PySide6",
        "PyQt6.QtPdf",
        "PyQt6.QtSvg",
        "numpy",
        "scipy",
        "pandas",
        "matplotlib",
        "PIL",
        "Pillow",
        "IPython",
        "jupyter",
        "pytest",
        "_pytest",
        "click",
        "werkzeug",
        "jinja2",
        "markupsafe",
        "comtypes.test",
        "pycaw.magic",
        "pycaw.callbacks",
    ],
    noarchive=False,
)
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="voice_input",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=False,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name="voice_input",
)
