"""Inventory direct translated calls that still resolve to original raw bytes."""
import argparse
import json
from pathlib import Path
import re

def audit(directory):
    labels={};calls={};taken={}
    for file in sorted(directory.glob('seg*.inc')):
        pending=[]
        for number,line in enumerate(file.read_text().splitlines(),1):
            match=re.fullmatch(r'(\w+):',line)
            if match:pending.append(match[1]);continue
            if not line or line.startswith((';','align ','%','section ')):continue
            for name in pending:labels[name]=(file.name,number,line)
            pending=[]
            match=re.fullmatch(r'CALL (loc_[0-9A-F]+)',line)
            if match:calls.setdefault(match[1],[]).append(f'{file.name}:{number}')
            match=re.match(r'(?:mov [^,]+, |PUSH32 )(loc_[0-9A-F]+)$',line)
            if match:taken.setdefault(match[1],[]).append(f'{file.name}:{number}')
    raw=[];missing=[]
    for name,sites in sorted(calls.items()):
        if name not in labels:missing.append({'target':name,'calls':sites});continue
        file,number,line=labels[name]
        if re.match(r'(?:db|dw|dd|times \d+ (?:db|dw|dd))\b',line):
            raw.append({'target':name,'definition':f'{file}:{number}','first_output':line,'calls':sites})
    candidates=[]
    for name,sites in sorted(taken.items()):
        if name not in labels:continue
        file,number,line=labels[name]
        if file in ['seg02.inc','seg03.inc','seg05.inc','seg07.inc'] and re.match(r'(?:db|dw|dd|times \d+ (?:db|dw|dd))\b',line):
            candidates.append({'target':name,'definition':f'{file}:{number}','first_output':line,'references':sites})
    return {'direct_call_targets':len(calls),'raw_direct_targets':raw,
            'external_or_missing_targets':missing,'raw_address_taken_candidates':candidates}

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('translation',type=Path)
    args=parser.parse_args();print(json.dumps(audit(args.translation),indent=2))
