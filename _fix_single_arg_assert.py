#!/usr/bin/env python3
"""Fix all single-argument kani::assert() calls by adding a message string."""

from pathlib import Path
import re

def fix_file(filepath: str) -> int:
    """Fix all single-argument kani::assert() calls in a file."""
    with open(filepath, 'r') as f:
        content = f.read()
    
    count = 0
    
    # Pattern to match kani::assert(predicate); where predicate can contain parentheses
    # We need to find kani::assert( followed by content and then );
    pattern = r'kani::assert\(([^)]*(?:\([^)]*\)[^)]*)*)\);'
    
    def replace_single_arg(m):
        nonlocal count
        # Check if there's already a message string (contains a comma)
        args = m.group(1)
        if ',' in args:
            return m.group(0)
        
        count += 1
        return f'kani::assert({args}, "assertion failed");'
    
    new_content = re.sub(pattern, replace_single_arg, content)
    
    if new_content != content:
        with open(filepath, 'w') as f:
            f.write(new_content)
    
    return count

def main():
    crates_dir = Path('/home/lewis/src/velvet-ballistics/crates')
    
    total = 0
    for rust_file in crates_dir.rglob('*.rs'):
        content = rust_file.read_text()
        if 'kani::assert(' not in content:
            continue
        if 'prop_kani::' in content:
            # Check for bare kani::assert( (not prop_kani::)
            has_bare = False
            for line in content.split('\n'):
                if 'kani::assert(' in line:
                    if 'prop_kani::' not in line:
                        has_bare = True
                        break
            if not has_bare:
                continue
        
        count = fix_file(str(rust_file))
        if count > 0:
            total += count
            print(f"{rust_file}: {count} replacements")
    
    print(f"\nTotal replacements: {total}")
    
    # Verify
    remaining = 0
    for rust_file in crates_dir.rglob('*.rs'):
        content = rust_file.read_text()
        # Check for single-argument kani::assert(
        for line in content.split('\n'):
            if 'kani::assert(' in line and 'prop_kani::' not in line:
                # Check if it has a message string
                if ', "' not in line and ", '" not in line:
                    # Could be single-argument
                    if re.search(r'kani::assert\([^)]*\);', line):
                        remaining += 1
    
    print(f"Remaining single-argument calls: {remaining}")
    if remaining == 0:
        print("STATUS: ALL FIXED")
    else:
        print("STATUS: PARTIAL")

if __name__ == '__main__':
    main()
