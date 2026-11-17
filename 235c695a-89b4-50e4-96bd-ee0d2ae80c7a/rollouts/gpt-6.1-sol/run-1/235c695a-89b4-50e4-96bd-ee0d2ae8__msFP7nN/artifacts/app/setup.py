#!/usr/bin/env python3
# Copyright (C) 2024 Efabless Corporation
#
# Permission to use, copy, modify, and/or distribute this software for any
# purpose with or without fee is hereby granted, provided that the above
# copyright notice and this permission notice appear in all copies.
#
# THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
# WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
# MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
# ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
# WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
# ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
# OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
import os
import re
import shlex
import shutil
from pathlib import Path
from setuptools import setup, Extension

import pybind11
from pybind11.setup_helpers import build_ext

__yosys_root__ = Path(__file__).parent

yosys_version_rx = re.compile(r'set\(YOSYS_VERSION "([\w\-\+\.]+)"\)')

with open(__yosys_root__ / "CMakeLists.txt", encoding="utf8") as f:
    # Extract and convert + to patch version
    version = yosys_version_rx.search(f.read())[1].replace("+", ".")


class libyosys_so_ext(Extension):
    def __init__(
        self,
    ) -> None:
        super().__init__(
            "libyosys.so",
            [],
        )

        # when iterating locally, you probably want to set this variable
        # to avoid mass rebuilds bec of pybind11's include path changing
        pybind_include = os.getenv("_FORCE_PYBIND_INCLUDE", pybind11.get_include())

        self.args = [
            "-DYOSYS_ENABLE_PYOSYS=ON",
            "-DYOSYS_WITHOUT_TCL=ON",
            "-DYOSYS_WITHOUT_READLINE=ON",
            "-DBUILD_TESTING=OFF",
        ]

    def custom_build(self, bext: build_ext):
        build_dir = Path(bext.build_temp).resolve() / "yosys"
        cpus = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else (os.cpu_count() or 1)
        quota = Path("/sys/fs/cgroup/cpu.max")
        if quota.exists():
            limit, period = quota.read_text().split()
            if limit != "max":
                cpus = min(cpus, max(1, int(limit) // int(period)))
        bext.spawn(["cmake", "-S", str(__yosys_root__.resolve()), "-B", str(build_dir),
                    "-G", "Ninja", f"-DPython3_EXECUTABLE={os.sys.executable}",
                    *self.args, *shlex.split(os.getenv("CMAKE_ARGS", ""))])
        bext.spawn(["cmake", "--build", str(build_dir), "--target", "yosys-abc", "-j" + str(cpus)])
        bext.spawn(["cmake", "--build", str(build_dir), "--target", "libyosys", "share", "-j" + str(cpus)])
        ext_fullpath = Path(bext.get_ext_fullpath(self.name))
        build_path = ext_fullpath.parents[1]
        pyosys_path = build_path / "pyosys"
        os.makedirs(pyosys_path, exist_ok=True)

        # libyosys.so
        target = pyosys_path / self.name
        shutil.copy(build_dir / self.name, target)
        bext.spawn(["strip", "-S", str(target)])

        # yosys-abc
        yosys_abc_target = pyosys_path / "yosys-abc"
        shutil.copy(build_dir / "yosys-abc", yosys_abc_target)
        bext.spawn(["strip", "-S", str(yosys_abc_target)])

        # share directory
        share_target = pyosys_path / "share"
        try:
            shutil.rmtree(share_target)
        except FileNotFoundError:
            pass

        shutil.copytree(build_dir / "share", share_target)


class custom_build_ext(build_ext):
    def build_extension(self, ext) -> None:
        if not hasattr(ext, "custom_build"):
            return super().build_extension(ext)
        return ext.custom_build(self)


with open(__yosys_root__ / "README.md", encoding="utf8") as f:
    long_description = f.read()

setup(
    name="pyosys",
    packages=["pyosys"],
    version=version,
    description="Python access to libyosys",
    long_description=long_description,
    long_description_content_type="text/markdown",
    license="MIT",
    classifiers=[
        "Programming Language :: Python :: 3",
        "Intended Audience :: Developers",
        "Operating System :: POSIX :: Linux",
        "Operating System :: MacOS :: MacOS X",
    ],
    python_requires=">=3.8",
    ext_modules=[libyosys_so_ext()],
    cmdclass={
        "build_ext": custom_build_ext,
    },
)
