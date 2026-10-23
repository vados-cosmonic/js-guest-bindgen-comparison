#!/usr/bin/env python3
"""Generate matching editable draw.io XML and standalone SVG call diagrams."""
from pathlib import Path
from html import escape
import xml.etree.ElementTree as E
OUT=Path(__file__).resolve().parents[1]/'docs/diagrams';OUT.mkdir(exist_ok=True)
pal={'host':('#dae8fc','#6c8ebf'),'native':('#d5e8d4','#82b366'),'js':('#fff2cc','#d6b656'),'note':('#f5f5f5','#666666'),'return':('#e1d5e7','#9673a6')}
data={
'a':('A · js-guest-bindgen',[
('Host calls a component export','Wasmtime lowers host values to the guest Canonical ABI','host'),
('Generated native core export','Rust (Boa) or C (Porffor) boxes raw scalar / pointer arguments','native'),
('Generated JS bindings lift arguments','Read canonical memory; native memory / UTF-8 helpers copy buffers','js'),
('Application JavaScript runs','Boa interprets JS bytecode · Porffor executes AOT-compiled Wasm','js'),
('Generated JS bindings lower the result','Allocate / write canonical result data through memory helpers','js'),
('Native wrapper returns → host lifts the result','Wasmtime invokes canonical post-return; tracked allocations are freed','return')],
['JS import wrapper lowers args → native core import → host implementation', 'Host result → JS binding lifts it → application continues'],
'Boa drains module jobs at initialization. Sync export wrappers do not pump an I/O loop.'),
'b':('B · specialized native runtime',[
('Host calls a component export','Wasmtime lowers host values to the guest Canonical ABI','host'),
('Generated native runtime receives the core call','wit-bindgen-c output lifts memory / flat values into typed C values','native'),
('Interface-specific native engine shim','Boxes typed values into Boa / Porffor values; no Canonical ABI layout work','native'),
('Application JavaScript runs','Boa interprets JS bytecode · Porffor executes AOT-compiled Wasm','js'),
('Shim unboxes result → native runtime lowers it','Engine API conversion in shim; canonical offsets and result area in runtime','native'),
('Host lifts result → canonical post-return','Generated runtime frees returned strings / lists; JS roots already released','return')],
['componentize-js JS import wrapper → specialized native interface shim', 'Generated native runtime lowers args → host → lifts result → JS'],
'Boa drains module jobs at initialization. No async WIT calls or P2 I/O poll loop in this world.'),
'c':('C · wit-dylib native runtime',[
('Host calls a component export','Wasmtime lowers host values to the guest Canonical ABI','host'),
('wit-dylib world.so receives the core call','export_start allocates call context; native Wasm lifts canonical arguments','native'),
('Generic native push_* callbacks','Root JS values on a call stack; composite types use WIT metadata','native'),
('export_call invokes application JavaScript','Boa interprets JS bytecode · Porffor executes AOT-compiled Wasm','js'),
('wit-dylib lowers result through pop_* callbacks','Generic native adapter extracts JS values; world.so handles canonical layout','native'),
('Host lifts result → canonical post-return','export_finish releases roots, temporary buffers, and call context','return')],
['componentize-js JS import wrapper → generic native call context', 'wit-dylib import function pointer lowers args → host → push_* lifts result'],
'Synchronous wit-dylib ABI only. No P3 task-return / waitables; no async loop on the measured path.')}
for key,(title,boxes,imports,note) in data.items():
 mx=E.Element('mxfile',host='app.diagrams.net');d=E.SubElement(mx,'diagram',name=title,id=key);model=E.SubElement(d,'mxGraphModel',dx='1200',dy='1000',grid='1',page='1',pageWidth='1100',pageHeight='1050');root=E.SubElement(model,'root');E.SubElement(root,'mxCell',id='0');E.SubElement(root,'mxCell',id='1',parent='0')
 svg=['<svg xmlns="http://www.w3.org/2000/svg" width="1100" height="1050" viewBox="0 0 1100 1050">','<defs><marker id="arrow" markerWidth="10" markerHeight="10" refX="9" refY="3" orient="auto"><path d="M0,0 L0,6 L9,3 z" fill="#566274"/></marker></defs>','<rect width="1100" height="1050" fill="white"/>',f'<text x="50" y="48" font-family="Arial,sans-serif" font-size="27" font-weight="bold" fill="#243746">{escape(title)}</text>']
 def box(id,x,y,w,h,lines,kind):
  fill,stroke=pal[kind];cell=E.SubElement(root,'mxCell',id=id,value='<br>'.join(map(escape,lines)),style=f'rounded=1;whiteSpace=wrap;html=1;fillColor={fill};strokeColor={stroke};fontSize=16;',vertex='1',parent='1');E.SubElement(cell,'mxGeometry',x=str(x),y=str(y),width=str(w),height=str(h),attrib={'as':'geometry'})
  svg.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="9" fill="{fill}" stroke="{stroke}" stroke-width="2"/>')
  for j,line in enumerate(lines):svg.append(f'<text x="{x+w/2}" y="{y+29+j*24}" text-anchor="middle" font-family="Arial,sans-serif" font-size="{18 if j==0 else 15}" font-weight="{"bold" if j==0 else "normal"}" fill="#243746">{escape(line)}</text>')
 for i,(label,detail,kind) in enumerate(boxes):
  y=80+i*113;box(str(i+2),75,y,950,82,[str(i+1)+'. '+label,detail],kind)
  if i:
   cell=E.SubElement(root,'mxCell',id='e'+str(i),style='edgeStyle=orthogonalEdgeStyle;rounded=0;html=1;endArrow=block;',edge='1',parent='1',source=str(i+1),target=str(i+2));E.SubElement(cell,'mxGeometry',relative='1',attrib={'as':'geometry'})
   svg.append(f'<path d="M550,{y-31} L550,{y-5}" stroke="#566274" stroke-width="2" marker-end="url(#arrow)"/>')
 box('imports',75,779,950,110,['Nested host import (inside step 4)',*imports],'host')
 box('jobs',75,916,950,90,['Jobs and asynchronous execution',note],'note')
 svg.append('</svg>');(OUT/f'{key}-call-path.svg').write_text('\n'.join(svg));E.indent(mx);E.ElementTree(mx).write(OUT/f'{key}-call-path.drawio',encoding='utf-8',xml_declaration=True)
