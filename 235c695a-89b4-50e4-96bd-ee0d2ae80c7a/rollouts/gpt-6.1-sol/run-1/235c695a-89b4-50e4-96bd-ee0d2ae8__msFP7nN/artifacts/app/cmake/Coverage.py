"""Collect the instrumented build's coverage report."""
import os
from pathlib import Path
import shutil
import subprocess
import sys

source, build, action = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
if action == 'clean':
    if (source / 'coverage').exists():
        shutil.rmtree(source / 'coverage')
    (build / 'yosys.profdata').unlink(missing_ok=True)
else:
    env = dict(os.environ, LLVM_PROFILE_FILE=str(source / 'coverage/coverage_%p.profraw'))
    subprocess.run([str(build / 'yosys'), '-qp', 'help; help -all'], env=env, check=True)
    if (source / 'coverage_html').exists():
        shutil.rmtree(source / 'coverage_html')
    subprocess.run(['llvm-profdata', 'merge', '-sparse', *map(str, (source / 'coverage').glob('coverage_*.profraw')), '-o', str(build / 'yosys.profdata')], check=True)
    subprocess.run(['llvm-cov', 'show', str(build / 'yosys'), '-instr-profile=' + str(build / 'yosys.profdata'), '-format=html', '-output-dir=' + str(source / 'coverage_html'), '--compilation-dir=' + str(source), '-ignore-filename-regex=(^|.*/)libs/.*|/usr/include/.*'], check=True)
