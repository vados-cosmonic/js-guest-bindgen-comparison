#!/usr/bin/env python3
"""Fail if artifacts drift from the requested synchronous P2 world."""
import pathlib,subprocess,json,re
root=pathlib.Path(__file__).resolve().parents[1]
records={}
for engine in ['boa','porffor']:
 for approach in 'abc':
  cell=f'{engine}-{approach}';file=root/'build'/cell/'component.wasm'
  subprocess.run(['wasm-tools','validate',str(file)],check=True)
  wit=subprocess.check_output(['wasm-tools','component','wit',str(file)],text=True)
  assert 'async func' not in wit and 'future<' not in wit and 'stream<' not in wit,cell
  assert not re.search(r'wasi:[^\s;]*@0\.3',wit),cell
  for name in ['scalar','floating','text','bytes','structured','imports','import-text']:
   assert re.search(r'export '+name+r': func',wit), (cell,name)
  (root/'results'/f'{cell}-world.wit').write_text(wit)
  records[cell]={'valid':True,'p2_sync_world':True,'exports':7}
(root/'results/validation.json').write_text(json.dumps(records,indent=2)+'\n')
print('Validated six components and their synchronous P2 interfaces.')
