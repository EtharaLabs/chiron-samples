"""Probe inputs written fresh for every run, and the generated number and boundary cases."""

from __future__ import annotations

import gzip
import itertools
from pathlib import Path

EXAMPLE = """\
module top(input clk, input rst, input [7:0] a, input [7:0] b, input [1:0] sel,
           output reg [7:0] q, output [8:0] sum);
  assign sum = a + b;
  reg [7:0] pick;
  always @* begin
    case (sel)
      2'd0: pick = 8'h12;
      2'd1: pick = a;
      2'd2: pick = 8'h56;
      default: pick = 8'h78;
    endcase
  end
  always @(posedge clk)
    if (rst) q <= 8'd0; else q <= pick ^ b;
endmodule
"""

GLOB_A = "module leaf(input [3:0] x, output [3:0] y); assign y = ~x; endmodule\n"
GLOB_B = "module top2(input [3:0] i, output [3:0] o); leaf u(.x(i), .y(o)); endmodule\n"

PLUGIN = """\
#include "kernel/yosys.h"
USING_YOSYS_NAMESPACE
PRIVATE_NAMESPACE_BEGIN
struct GraderProbePass : public Pass {
	GraderProbePass() : Pass("grader_probe", "report that the plugin route works") { }
	void execute(std::vector<std::string>, RTLIL::Design *design) override {
		log("plugin route ok: %d modules\\n", GetSize(design->modules()));
	}
} GraderProbePass;
PRIVATE_NAMESPACE_END
"""

LIBERTY = """\
library(tiny) {
  cell(INV) { area: 1; pin(A) { direction: input; } pin(Y) { direction: output; function: "A'"; } }
  cell(NAND2) { area: 2;
    pin(A) { direction: input; } pin(B) { direction: input; }
    pin(Y) { direction: output; function: "(A B)'"; } }
  cell(DFF) { area: 4; ff(IQ, IQN) { clocked_on: C; next_state: D; }
    pin(C) { direction: input; clock: true; } pin(D) { direction: input; }
    pin(Q) { direction: output; function: "IQ"; } }
}
"""

BLIF = """\
.model t
.inputs a b c
.outputs y
.names a b n1
11 1
.names n1 c y
1- 1
-1 1
.end
"""

TCL = """\
puts "tcl [info tclversion]"
yosys read_verilog example.v
yosys proc
yosys stat
"""

PYOSYS_SMOKE = """\
from pyosys import libyosys as ys

design = ys.Design()
design.run_pass(["read_verilog", "example.v"])
design.run_pass("synth -top top")
design.run_pass("stat")
print("modules:", sorted(m.name.str() for m in design.modules_.values()) if hasattr(design, "modules_") else "n/a")
"""

DPI = """\
module top(output [31:0] y);
  import "DPI-C" function integer abs(integer x);
  localparam integer P = abs(-17);
  assign y = P;
endmodule
"""


def write_all(folder: Path) -> None:
    files = {
        "example.v": EXAMPLE,
        "plugin.cc": PLUGIN,
        "cells.lib": LIBERTY,
        "abc.blif": BLIF,
        "probe.tcl": TCL,
        "pyosys_smoke.py": PYOSYS_SMOKE,
        "dpi.v": DPI,
        "globdir/leaf.v": GLOB_A,
        "globdir/top2.v": GLOB_B,
    }
    for rel, text in files.items():
        path = folder / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    with open(folder / "example.v.gz", "wb") as raw:
        with gzip.GzipFile(filename="example.v", mode="wb", fileobj=raw, mtime=0) as handle:
            handle.write(EXAMPLE.encode())
    for name, text in number_sources():
        (folder / "numbers").mkdir(exist_ok=True)
        (folder / "numbers" / f"{name}.sv").write_text(text, encoding="utf-8")


def feature_probes(yosys: str, folder: Path, triplet: str) -> list[tuple[str, list[str]]]:
    q = [yosys, "-Q", "-T"]
    libc = f"/lib/{triplet}/libc.so.6"
    return [
        ("tcl", q + ["-c", "probe.tcl"]),
        ("zlib-read", q + ["-p", "read_verilog example.v.gz; hierarchy -top top; stat"]),
        ("zlib-write", q + ["-p", "read_verilog example.v; write_rtlil roundtrip.il.gz; design -reset; read_rtlil roundtrip.il.gz; stat"]),
        ("glob", q + ["-p", "read_verilog globdir/*.v; hierarchy -top top2; flatten; stat"]),
        ("abc", q + ["-p", "read_verilog example.v; synth -top top; abc -g AND,NAND,XOR; stat"]),
        ("spawn", q + ["-p", "!echo spawn-probe"]),
        ("debug-log", q + ["-p", "read_verilog example.v; debug proc_rom"]),
        ("dpi", q + ["-p", f"plugin -a abs -i {libc}; read_verilog -sv dpi.v; proc; opt; write_verilog -noattr"]),
        ("xtrace", q + ["-X", "-X", "-p", "read_verilog example.v; proc"]),
        ("share-lookup", q + ["-p", "read_verilog -lib +/simcells.v; ls"]),
    ]


SEEDS = [
    ("add", "a + b"), ("sub", "a - b"), ("mul", "a * b"), ("div", "a / b"), ("mod", "a % b"),
    ("pow", "a ** 3"), ("shl", "a << 5"), ("ashr", "$signed(a) >>> 3"), ("cmp", "{a < b, a == b, a > b}"),
    ("concat", "{a, b, ~a}"), ("repl", "{4{a[3:0]}}"), ("ternary", "a > b ? a - b : b - a"),
    ("clog2", "$clog2(a + 1)"), ("signext", "$signed(a[7:0])"), ("reduce", "{&a, |a, ^a}"),
]
VALUES = [(0, 1), (255, 1), (127, 128), (65535, 3), (-1, 7)]
WIDTHS = [8, 32, 64, 128]


def number_sources() -> list[tuple[str, str]]:
    """Seed expressions mutated over operand values and result widths."""
    out = []
    for (name, expr), (av, bv), width in itertools.product(SEEDS, VALUES, WIDTHS):
        if width == 128 and name in ("div", "mod", "pow"):
            continue
        a = f"-{width}'sd{-av}" if av < 0 else f"{width}'d{av}"
        b = f"{width}'d{bv}"
        body = (
            f"module top(output [{width - 1}:0] y);\n"
            f"  localparam logic signed [{width - 1}:0] a = {a};\n"
            f"  localparam logic [{width - 1}:0] b = {b};\n"
            f"  assign y = {expr};\nendmodule\n"
        )
        out.append((f"{name}_w{width}_a{'m' if av < 0 else ''}{abs(av)}_b{bv}", body))
    out += [
        ("real_div", "module top(output [63:0] y);\n  localparam real R = 1.0 / 3.0;\n  assign y = $realtobits(R);\nendmodule\n"),
        ("real_rtoi", "module top(output [31:0] y);\n  localparam real R = 2.5e3 * 1.5;\n  assign y = $rtoi(R);\nendmodule\n"),
        ("real_small", "module top(output [63:0] y);\n  localparam real R = 1.0e-300 * 1.0e-10;\n  assign y = $realtobits(R);\nendmodule\n"),
        ("string_utf8", "module top(output [79:0] y);\n  localparam S = \"h\\303\\251llo\";\n  assign y = S;\nendmodule\n"),
        ("wide_ones", "module top(output [1023:0] y);\n  assign y = {1024{1'b1}} >> 7;\nendmodule\n"),
        ("big_pow", "module top(output [127:0] y);\n  assign y = 128'd2 ** 100 + 1;\nendmodule\n"),
        ("neg_mod", "module top(output signed [15:0] y);\n  assign y = -16'sd7 % 16'sd3;\nendmodule\n"),
        ("div_zero", "module top(output [7:0] y);\n  assign y = 8'd5 / 8'd0;\nendmodule\n"),
    ]
    return out


def number_cases(folder: Path) -> list[tuple[str, str]]:
    return [
        (name, f"read_verilog -sv numbers/{name}.sv; proc; opt -purge; write_verilog -noattr")
        for name, _ in number_sources()
    ]
