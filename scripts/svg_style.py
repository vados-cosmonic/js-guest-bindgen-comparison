"""Shared accessible light/dark palettes for standalone SVG documentation."""
from html import escape

TEXT = "#243746"
MUTED = "#526477"
BACKGROUND = "#ffffff"
GRID = "#d7dee7"
# Fill, border, dark fill, dark border. Each color denotes a code layer.
PALETTE = {
    "host": ("#dceafb", "#4776aa", "#203c59", "#85b5e8"),
    "bindings": ("#fff0c2", "#96700c", "#4b3a13", "#e8c76b"),
    "runtime": ("#eadffa", "#8253a7", "#3f2c53", "#c1a0e5"),
    "adapter": ("#d7efdf", "#387b53", "#203f30", "#8bcda6"),
    "engine": ("#fde1d7", "#aa6243", "#4c3027", "#edaa8d"),
    "glue": ("#d7f0f0", "#347d83", "#1d4045", "#84cbd0"),
    "note": ("#eef1f5", "#7b8797", "#283343", "#99a8ba"),
}


def css():
    rules = [
        "svg { color-scheme: light dark; }",
        f".canvas {{ fill: {BACKGROUND}; }}",
        f"text {{ fill: {TEXT}; font-family: Arial, sans-serif; }}",
        f".muted {{ fill: {MUTED}; }}",
        f".arrow {{ stroke: {MUTED}; fill: none; }}",
        f".arrowhead {{ fill: {MUTED}; }}",
    ]
    for key, (fill, border, _, _) in PALETTE.items():
        rules.append(f".layer-{key} {{ fill: {fill}; stroke: {border}; }}")
    # Matplotlib emits inline styles; override exact palette values in dark mode.
    replacements = {BACKGROUND: "#101821", TEXT: "#eef3f8", MUTED: "#b8c5d4", GRID: "#344456"}
    for fill, border, dark_fill, dark_border in PALETTE.values():
        replacements[fill] = dark_fill
        replacements[border] = dark_border
    dark = [
        ".canvas { fill: #101821; }", "text { fill: #eef3f8; }",
        ".muted { fill: #b8c5d4; }", ".arrow { stroke: #b8c5d4; }",
        ".arrowhead { fill: #b8c5d4; }",
    ]
    for key, (_, _, fill, border) in PALETTE.items():
        dark.append(f".layer-{key} {{ fill: {fill}; stroke: {border}; }}")
    for light, night in replacements.items():
        for prop in ("fill", "stroke"):
            dark.append(f'[style*="{prop}: {light}"] {{ {prop}: {night} !important; }}')
    return "\n".join(rules) + "\n@media (prefers-color-scheme: dark) {\n" + "\n".join(dark) + "\n}"


def start_svg(width, height, title, description):
    return [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
        f'<title id="title">{escape(title)}</title>',
        f'<desc id="desc">{escape(description)}</desc>',
        f"<style>{css()}</style>",
        '<defs><marker id="arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto"><path d="M0,0 L0,6 L9,3 z" class="arrowhead"/></marker></defs>',
        f'<rect class="canvas" width="{width}" height="{height}"/>',
    ]
