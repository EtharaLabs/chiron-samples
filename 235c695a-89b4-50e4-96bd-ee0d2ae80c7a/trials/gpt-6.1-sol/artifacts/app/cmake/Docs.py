"""Prepare the documentation inputs using the built programs."""
import pathlib
import shutil
import subprocess
import sys

source, build, prefix = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
out = source / 'docs/source/generated'
out.mkdir(parents=True, exist_ok=True)
yosys = str(build / (prefix + 'yosys'))
for kind in ('cmds', 'cells'):
    subprocess.run([yosys, '-p', f'help -dump-{kind}-json {out / (kind + ".json")}'], check=True)
for name in ('yosys', 'yosys-smtbmc', 'yosys-witness', 'yosys-config', 'yosys-filterlib', 'yosys-abc'):
    program = build / (prefix + name)
    if not program.exists():
        continue
    stderr = name in ('yosys-filterlib', 'yosys-abc')
    with (out / name).open('w') as f:
        result = subprocess.run([str(program), '--help'], **({'stderr': f} if stderr else {'stdout': f}))
    if result.returncode and not stderr:
        raise subprocess.CalledProcessError(result.returncode, result.args)
(out / 'functional').mkdir(exist_ok=True)
shutil.copy(source / 'backends/functional/smtlib.cc', out / 'functional/smtlib.cc')
with (out / 'functional/rosette.diff').open('w') as f:
    result = subprocess.run(['diff', '-U', '20', str(source / 'backends/functional/smtlib.cc'), str(source / 'backends/functional/smtlib_rosette.cc')], stdout=f)
    if result.returncode not in (0, 1):
        raise subprocess.CalledProcessError(result.returncode, result.args)
shutil.copy(source / 'passes/cmds/chformal.cc', out / 'chformal.cc')
subprocess.run([yosys, '-qQT', '-h', 'chformal', '-l', str(out / 'chformal.log')], check=True)
