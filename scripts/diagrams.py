#!/usr/bin/env python3
"""Generate SVG call paths and matching editable draw.io documents."""
from html import escape
from pathlib import Path
import xml.etree.ElementTree as E
from svg_style import PALETTE, start_svg

OUT = Path(__file__).resolve().parents[1] / "docs/diagrams"
OUT.mkdir(exist_ok=True)
DATA = {
    "a": {
        "title": "A · js-guest-bindgen",
        "boxes": [
            ("Host calls a component export", "Wasmtime lowers host values to the guest Canonical ABI", "host"),
            ("Generated native core wrapper receives the call", "wit_export_i · Rust on Boa / C on Porffor · boxes raw core arguments", "adapter"),
            ("Generated JS bindings lift arguments", "js-guest-bindgen · reads canonical memory using native memory / UTF-8 helpers", "bindings"),
            ("Application JavaScript runs", "Boa interprets JS bytecode · Porffor executes AOT-compiled Wasm", "engine"),
            ("Generated JS bindings lower the result", "js-guest-bindgen · allocates and writes canonical result data", "bindings"),
            ("Native core wrapper returns", "Unboxes the raw core result · keeps returned allocations alive until post-return", "adapter"),
            ("Host lifts the result and invokes canonical post-return", "Wasmtime owns this phase; the native A wrapper frees its tracked result allocations", "host"),
        ],
        "imports": [("JS binding", "Lowers arguments", "bindings"), ("Native wrapper", "Calls the core import", "adapter"), ("Host function", "Produces the result", "host")],
        "return": "Result returns through the native wrapper; generated JS bindings lift it for the application.",
        "jobs": "Boa drains module jobs at initialization. Synchronous exports do not pump an I/O loop.",
        "legend": [("host", "Wasmtime / host boundary"), ("bindings", "js-guest-bindgen generated JS"), ("adapter", "Native engine wrappers · this repo"), ("engine", "Application running in Boa / Porffor")],
    },
    "b": {
        "title": "B · new componentize-js + native engine shims",
        "boxes": [
            ("Host calls a component export", "Wasmtime lowers host values to the guest Canonical ABI", "host"),
            ("Generated native ABI runtime receives the core call", "wit-bindgen-c output · lifts canonical memory / flat values into typed C values", "runtime"),
            ("Interface-specific native engine shim boxes arguments", "native/common/shims_b.c · uses Boa / Porffor APIs; no canonical memory layout work", "adapter"),
            ("Application JavaScript runs", "Boa interprets JS bytecode · Porffor executes AOT-compiled Wasm", "engine"),
            ("Native engine shim unboxes the application result", "Extracts typed native values through engine APIs · releases temporary JS roots", "adapter"),
            ("Generated native ABI runtime lowers the result", "wit-bindgen-c output · writes canonical offsets, flat values and return areas", "runtime"),
            ("Host lifts the result and invokes canonical post-return", "Wasmtime owns this phase; the generated native runtime frees returned strings / lists", "host"),
        ],
        "imports": [("componentize-js JS", "Generated import module", "glue"), ("Native typed shim", "Engine value conversion", "adapter"), ("Native ABI runtime", "wit-bindgen-c lowering", "runtime"), ("Host function", "Produces the result", "host")],
        "return": "Result returns through the native ABI runtime and shim to the generated JS import wrapper.",
        "jobs": "Boa drains module jobs at initialization. No async WIT calls or P2 I/O poll loop run here.",
        "legend": [("host", "Wasmtime / host boundary"), ("glue", "New componentize-js generated JS"), ("runtime", "Native ABI runtime · wit-bindgen-c"), ("adapter", "Native engine shims · this repo"), ("engine", "Application running in Boa / Porffor")],
    },
    "c": {
        "title": "C · new componentize-js targeting wit-dylib",
        "boxes": [
            ("Host calls a component export", "Wasmtime lowers host values to the guest Canonical ABI", "host"),
            ("wit-dylib world.so receives the core call", "Calls export_start for a context, then lifts canonical arguments through native Wasm", "runtime"),
            ("Generic native push_* callbacks create engine values", "native/common/runtime_c.c · roots values on a call stack; composites use WIT metadata", "adapter"),
            ("export_call invokes application JavaScript", "Boa interprets JS bytecode · Porffor executes AOT-compiled Wasm", "engine"),
            ("Generic native pop_* callbacks extract the result", "Native engine adapter converts JS values using the uniform wit-dylib contract", "adapter"),
            ("wit-dylib world.so lowers the result", "Generated Wasm owns canonical memory layout, flat values and return areas", "runtime"),
            ("Host lifts the result and invokes canonical post-return", "Wasmtime owns this phase; export_finish in the adapter frees roots, buffers and context", "host"),
        ],
        "imports": [("componentize-js JS", "Generated import module", "glue"), ("Generic adapter", "Native call context", "adapter"), ("wit-dylib world.so", "Import pointer / lowering", "runtime"), ("Host function", "Produces the result", "host")],
        "return": "wit-dylib lifts the host result through push_* callbacks; the JS import wrapper returns it.",
        "jobs": "Synchronous P2 only. No P3 task-return / waitables or async I/O loop on the measured path.",
        "legend": [("host", "Wasmtime / host boundary"), ("glue", "New componentize-js generated JS"), ("runtime", "Native ABI runtime · wit-dylib"), ("adapter", "Native generic engine adapter · this repo"), ("engine", "Application running in Boa / Porffor")],
    },
}


def generate(key, data):
    width, height = 1100, 1425
    mx = E.Element("mxfile", host="app.diagrams.net")
    diagram = E.SubElement(mx, "diagram", name=data["title"], id=key)
    model = E.SubElement(diagram, "mxGraphModel", grid="1", page="1", pageWidth=str(width), pageHeight=str(height))
    root = E.SubElement(model, "root")
    E.SubElement(root, "mxCell", id="0")
    E.SubElement(root, "mxCell", id="1", parent="0")
    svg = start_svg(width, height, data["title"], "Export and nested import call paths. Colors identify code layers; the legend is inside the diagram. " + data["jobs"])

    def box(id, x, y, w, h, lines, layer, font=18):
        fill, stroke, _, _ = PALETTE[layer]
        cell = E.SubElement(root, "mxCell", id=id, value="<br>".join(map(escape, lines)), style=f"rounded=1;whiteSpace=wrap;html=1;fillColor={fill};strokeColor={stroke};fontColor=#243746;fontSize=16;", vertex="1", parent="1")
        E.SubElement(cell, "mxGeometry", x=str(x), y=str(y), width=str(w), height=str(h), attrib={"as": "geometry"})
        svg.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="9" class="layer-{layer}" stroke-width="2"/>')
        for j, line in enumerate(lines):
            baseline = y + (23 if h == 35 else 29) + j * 24
            svg.append(f'<text x="{x+w/2}" y="{baseline}" text-anchor="middle" font-size="{font if j == 0 else 15}" font-weight="{"bold" if j == 0 else "normal"}">{escape(line)}</text>')

    def arrow(id, source, target, x1, y1, x2, y2):
        cell = E.SubElement(root, "mxCell", id=id, style="edgeStyle=orthogonalEdgeStyle;rounded=0;html=1;endArrow=block;", edge="1", parent="1", source=source, target=target)
        E.SubElement(cell, "mxGeometry", relative="1", attrib={"as": "geometry"})
        svg.append(f'<path d="M{x1},{y1} L{x2},{y2}" class="arrow" stroke-width="2" marker-end="url(#arrow)"/>')

    box("title", 45, 12, 1010, 55, [data["title"]], "note", 26)
    for i, (title, detail, layer) in enumerate(data["boxes"]):
        y = 90 + i * 109
        box(f"step{i}", 65, y, 970, 79, [f"{i+1}. {title}", detail], layer)
        if i:
            arrow(f"edge{i}", f"step{i-1}", f"step{i}", 550, y-30, 550, y-5)
    box("import-title", 65, 870, 970, 54, ["Nested host import · called from the application in step 4"], "note")
    imports = data["imports"]
    w = (970 - (len(imports)-1)*30) / len(imports)
    for i, (title, detail, layer) in enumerate(imports):
        x = 65 + i * (w+30)
        box(f"import{i}", x, 940, w, 78, [title, detail], layer, 17)
        if i:
            arrow(f"import-edge{i}", f"import{i-1}", f"import{i}", x-30, 979, x-5, 979)
    box("return-note", 65, 1034, 970, 79, ["The host result returns to the active application call", data["return"]], "note")
    box("jobs", 65, 1134, 970, 79, ["Jobs and asynchronous execution", data["jobs"]], "note")
    svg.append('<text x="65" y="1253" font-size="18" font-weight="bold">Color legend · code layer owning each step</text>')
    legend = data["legend"] + [("note", "Execution notes / cross-layer explanation")]
    for i, (layer, label) in enumerate(legend):
        x = 65 + (i % 2) * 500
        y = 1270 + (i // 2) * 43
        box(f"legend{i}", x, y, 470, 35, [label], layer, 15)
    svg.append("</svg>")
    (OUT / f"{key}-call-path.svg").write_text("\n".join(svg) + "\n")
    E.indent(mx)
    E.ElementTree(mx).write(OUT / f"{key}-call-path.drawio", encoding="utf-8", xml_declaration=True)


if __name__ == "__main__":
    for key, data in DATA.items():
        generate(key, data)
