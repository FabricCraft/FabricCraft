#!/usr/bin/env python3
"""Select five actual T2P records from a locally obtained CSV for permitted private research.

No network access; never commits data. Requires explicit user-supplied CSV.
"""
import argparse
import csv
from pathlib import Path

FAMILIES = ('denim', 'jersey', 'chiffon', 'satin', 'twill')
RESEARCH_DIR = Path(__file__).resolve().parents[1] / 'materials' / 'local-research'

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('csv_path', type=Path, help='Local T2P CSV obtained under its license')
    ap.add_argument('--out', type=Path, default=RESEARCH_DIR / 't2p-five.csv',
                    help='Output path inside the ignored materials/local-research directory')
    args = ap.parse_args()
    output = args.out.resolve()
    if RESEARCH_DIR not in output.parents:
        ap.error('Output must be inside materials/local-research in this checkout')
    if args.csv_path.resolve() == output:
        ap.error('Input and output must differ')
    if output.exists() and output.stat().st_nlink > 1:
        ap.error('Output has other hard links; choose a private file')
    with args.csv_path.open(newline='', encoding='utf-8-sig') as f:
        reader = csv.DictReader(f)
        if not reader.fieldnames or 'Fabric Family' not in reader.fieldnames:
            ap.error('Expected T2P column: Fabric Family')
        selected = {}
        for row in reader:
            family = (row.get('Fabric Family') or '').casefold().strip()
            for target in FAMILIES:
                if target not in selected and target in family:
                    selected[target] = row
        fields = reader.fieldnames
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fields)
        writer.writeheader()
        writer.writerows(selected[x] for x in FAMILIES if x in selected)
    print('Private research file:', output)
    for family in FAMILIES:
        row = selected.get(family)
        print(f'{family}: ' + (f"itemId={row.get('itemId','?')} name={row.get('name','?')}" if row else 'NOT FOUND'))
    print('Research-only T2P rows: do not commit or redistribute.')

if __name__ == '__main__':
    main()
