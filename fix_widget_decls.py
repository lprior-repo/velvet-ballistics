#!/usr/bin/env python3
"""
Fix widget declaration syntax: `= WidgetType{` → `:= WidgetType{`
for instance declarations (NOT template definitions).
"""

import re
import sys

# Known Makepad widget types that can be on the right side of a declaration
WIDGET_TYPES = {
    'View', 'Label', 'ButtonFlat', 'ButtonFlatter', 'Button', 'CheckBox', 'Slider',
    'DropDown', 'Video', 'Audio', 'TextInput', 'Hr', 'Filler',
    'ScrollXYView', 'ScrollXView', 'ScrollYView',
    'TransportBtn', 'JumpChip', 'EventChip', 'NodeCard', 'InfoCard',
    'CertPanel', 'ShardCard', 'AlertCard', 'SubTab', 'SubTabActive',
    'Window', 'Root', 'PageFlip',
}

def main():
    filepath = sys.argv[1] if len(sys.argv) > 1 else "main.rs"

    with open(filepath, 'r') as f:
        content = f.read()
    lines = content.split('\n')

    changes_made = 0
    new_lines = []

    for i, line in enumerate(lines):
        original_line = line

        # Skip template definitions: "let WidgetName = WidgetType{"
        if re.match(r'^\s*let\s+\w+\s*=\s*\w+\s*\{', line):
            new_lines.append(line)
            continue

        # Check for instance declaration: whitespace + name = WidgetType{
        # or: whitespace + name= WidgetType{  (no space around =)
        # where WidgetType is a known Makepad widget type
        # Pattern variations:
        #   "    name = Label{"  or "    name= Label{" or "    name  =  Label{"
        match = re.match(r'^(\s+)(\w+)\s*=\s*(\w+)(\s*)(\{.*)', line)
        if match:
            indent = match.group(1)
            widget_name = match.group(2)
            widget_type = match.group(3)
            space_after_type = match.group(4)
            rest = match.group(5)  # Everything after the first {

            # Only change if widget_type is a known Makepad widget
            if widget_type in WIDGET_TYPES:
                # Change = to := but preserve everything after
                new_line = indent + widget_name + " :=" + space_after_type + widget_type + rest
                new_lines.append(new_line)
                changes_made += 1
                print(f"LINE {i+1}: '{widget_name}' ({widget_type}) -> ':='")
                continue

        new_lines.append(line)

    print(f"\nTotal changes made: {changes_made}")

    # Write back
    with open(filepath, 'w') as f:
        f.write('\n'.join(new_lines))

    print(f"Written to {filepath}")

if __name__ == "__main__":
    main()