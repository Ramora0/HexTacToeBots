"""Build the minimax C++ extension.

Usage:  cd bots/SealBot && python setup.py build_ext --inplace
"""

import platform
from pybind11.setup_helpers import Pybind11Extension, build_ext
from setuptools import setup

if platform.system() == "Windows":
    extra_compile_args = ["/O2", "/DNDEBUG"]
else:
    extra_compile_args = ["-O3", "-march=native", "-DNDEBUG"]

setup(
    name="minimax_cpp",
    ext_modules=[
        Pybind11Extension("minimax_cpp", ["minimax_bot.cpp"],
                          cxx_std=17,
                          extra_compile_args=extra_compile_args,
                          include_dirs=["."]),
    ],
    cmdclass={"build_ext": build_ext},
)
