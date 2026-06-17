#!/usr/bin/env python3
"""Fix all single-argument kani::assert() calls by adding a message string.

This script handles complex predicates that contain parentheses and commas.
"""

from pathlib import Path
import re

def find_matching_paren(s: str, start: int) -> int:
    """Find the index of the matching closing parenthesis."""
    depth = 0
    for i in range(start, len(s)):
        if s[i] == '(':
            depth += 1
        elif s[i] == ')':
            depth -= 1
            if depth == 0:
                return i
    return -1

def fix_file(filepath: str) -> int:
    """Fix all single-argument kani::assert() calls in a file."""
    with open(filepath, 'r') as f:
        content = f.read()
    
    count = 0
    
    # Find all kani::assert( calls and check if they have a message string
    pattern = r'kani::assert\('
    result = []
    i = 0
    
    while i < len(content):
        match = re.search(pattern, content[i:])
        if not match:
            result.append(content[i:])
            break
        
        # Add everything before the match
        result.append(content[i:i+match.start()])
        
        # Find the matching closing paren
        open_pos = i + match.start() + len('kani::assert(')
        close_pos = find_matching_paren(content, open_pos)
        
        if close_pos == -1:
            # No matching paren, keep as-is
            result.append(content[i:i+match.end()])
            i += match.end()
            continue
        
        # Extract the argument
        arg = content[open_pos:close_pos].strip()
        
        # Check if this is a single-argument call (no comma at top level)
        # We need to check if there's a comma at depth 0
        has_comma = False
        depth = 0
        in_string = False
        for c in arg:
            if in_string:
                if c == '"':
                    in_string = False
            elif c == '"':
                in_string = True
            elif c == '(':
                depth += 1
            elif c == ')':
                depth -= 1
            elif c == ',' and depth == 0:
                has_comma = True
                break
        
        if not has_comma:
            # Single-argument call - add message string
            count += 1
            result.append(f'kani::assert({arg}, "assertion failed")')
        else:
            # Multi-argument call - keep as-is
            result.append(content[i:close_pos+1])
        
        i = close_pos + 1
    
    if count > 0:
        with open(filepath, 'w') as f:
            f.write(''.join(result))
    
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
        pattern = r'kani::assert\('
        i = 0
        while i < len(content):
            match = re.search(pattern, content[i:])
            if not match:
                break
            open_pos = i + match.start() + len('kani::assert(')
            close_pos = find_matching_paren(content, open_pos)
            if close_pos == -1:
                break
            arg = content[open_pos:close_pos].strip()
            
            has_comma = False
            depth = 0
            in_string = False
            for c in arg:
                if in_string:
                    if c == '"':
                        in_string = False
                elif c == '"':
                    in_string = True
                elif c == '(':
                    depth += 1
                elif c == ')':
                    depth -= 1
                elif c == ',' and depth == 0:
                    has_comma = True
                    break
            
            if not has_comma:
                remaining += 1
            
            i = close_pos + 1
    
    print(f"Remaining single-argument calls: {remaining}")
    if remaining == 0:
        print("STATUS: ALL FIXED")
    else:
        print("STATUS: PARTIAL")

if __name__ == '__main__':
    main()
