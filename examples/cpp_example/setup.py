from pybind11.setup_helpers import Pybind11Extension, build_ext
from setuptools import setup

setup(
    name="my_bot_cpp",
    ext_modules=[
        Pybind11Extension("my_bot_cpp", ["my_bot.cpp"],
                          cxx_std=17,
                          extra_compile_args=["-O3", "-march=native"],
                          include_dirs=["."]),
    ],
    cmdclass={"build_ext": build_ext},
)
