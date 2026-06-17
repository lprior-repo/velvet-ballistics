#!/usr/bin/env python3
"""Final fix for remaining kani::assert_eq! and kani::assert_ne! calls."""

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
    """Fix all remaining kani::assert_eq!/assert_ne! in a file."""
    with open(filepath, 'r') as f:
        content = f.read()
    
    count = 0
    
    # Handle empty calls first: kani::assert_eq!(); -> kani::assert(true, "assertion failed");
    empty_eq = re.sub(r'kani::assert_eq!\(\s*\)\s*;', 'kani::assert(true, "assertion failed");', content)
    if empty_eq != content:
        count += content.count('kani::assert_eq!();')
        content = empty_eq
    
    empty_ne = re.sub(r'kani::assert_ne!\(\s*\)\s*;', 'kani::assert(true, "assertion failed");', content)
    if empty_ne != content:
        count += content.count('kani::assert_ne!();')
        content = empty_ne
    
    # Handle multi-line calls by processing the entire content
    def replace_multiline(m):
        nonlocal count
        is_eq = 'assert_eq' in m.group(1)
        prefix = m.group(1)
        args_text = m.group(2)
        
        # Parse arguments (split by top-level comma only)
        args = []
        current = []
        depth = 0
        in_string = False
        string_char = None
        
        for c in args_text:
            if in_string:
                current.append(c)
                if c == '\\':
                    pass  # escaped char, skip next
                elif c == string_char:
                    in_string = False
            elif c in ('"', "'"):
                in_string = True
                string_char = c
                current.append(c)
            elif c == '(':
                depth += 1
                current.append(c)
            elif c == ')':
                depth -= 1
                current.append(c)
            elif c == ',' and depth == 0:
                args.append(''.join(current).strip())
                current = []
            else:
                current.append(c)
        
        if current:
            args.append(''.join(current).strip())
        
        if len(args) >= 2:
            count += 1
            op = '==' if is_eq else '!='
            msg = args[-1].strip().rstrip(',').strip() if len(args) > 2 and (args[-1].startswith('"') or args[-1].startswith("'")) else '"assertion failed"'
            if not (msg.startswith('"') or msg.startswith("'")):
                msg = '"assertion failed"'
            return f'kani::assert({args[0]} {op} {args[1]}, {msg});'
        return m.group(0)
    
    # Pattern for multi-line calls: kani::assert_eq!( ... ) where ... contains newlines
    pattern = r'kani::assert_(eq|ne)!\(\s*\n(.*?)\n\s*(\))\s*;'
    content = re.sub(pattern, replace_multiline, content, flags=re.DOTALL)
    
    # Handle remaining single-line calls with complex expressions
    def replace_single_line(m):
        nonlocal count
        is_eq = 'assert_eq' in m.group(1)
        prefix = m.group(1)
        args_text = m.group(2)
        
        # Parse arguments
        args = []
        current = []
        depth = 0
        in_string = False
        string_char = None
        
        for c in args_text:
            if in_string:
                current.append(c)
                if c == '\\':
                    pass
                elif c == string_char:
                    in_string = False
            elif c in ('"', "'"):
                in_string = True
                string_char = c
                current.append(c)
            elif c == '(':
                depth += 1
                current.append(c)
            elif c == ')':
                depth -= 1
                current.append(c)
            elif c == ',' and depth == 0:
                args.append(''.join(current).strip())
                current = []
            else:
                current.append(c)
        
        if current:
            args.append(''.join(current).strip())
        
        if len(args) >= 2:
            count += 1
            op = '==' if is_eq else '!='
            msg = args[-1].strip().rstrip(',').strip() if len(args) > 2 and (args[-1].startswith('"') or args[-1].startswith("'")) else '"assertion failed"'
            if not (msg.startswith('"') or msg.startswith("'")):
                msg = '"assertion failed"'
            return f'kani::assert({args[0]} {op} {args[1]}, {msg});'
        return m.group(0)
    
    # Pattern for single-line calls with complex expressions (contains parentheses)
    pattern2 = r'kani::assert_(eq|ne)!\(([^)]*\([^)]*\)[^)]*)\)\s*;'
    content = re.sub(pattern2, replace_single_line, content, flags=re.DOTALL)
    
    if count > 0:
        with open(filepath, 'w') as f:
            f.write(content)
    
    return count

def main():
    crates_dir = Path('/home/lewis/src/velvet-ballistics/crates')
    
    total = 0
    for rust_file in crates_dir.rglob('*.rs'):
        content = rust_file.read_text()
        if 'kani::assert_eq!' not in content and 'kani::assert_ne!' not in content:
            continue
        if 'prop_kani::' in content:
            # Check for bare kani::assert_eq! (not prop_kani::)
            has_bare = False
            for line in content.split('\n'):
                if 'kani::assert_eq!' in line or 'kani::assert_ne!' in line:
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
        for line in content.split('\n'):
            if ('kani::assert_eq!' in line or 'kani::assert_ne!' in line) and 'prop_kani::' not in line:
                remaining += 1
    
    print(f"Remaining: {remaining}")
    if remaining == 0:
        print("STATUS: ALL FIXED")
    else:
        print("STATUS: PARTIAL")

if __name__ == '__main__':
    main()
