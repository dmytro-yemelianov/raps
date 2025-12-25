#!/usr/bin/env python3
"""
Fix relative links in commands directory to use relative paths instead of absolute.
"""

import re
from pathlib import Path

def fix_links_in_file(file_path):
    """Fix links in a file to use relative paths."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original_content = content
        
        # Only process files in the commands directory
        if 'commands' not in str(file_path):
            return False
        
        # Pattern to match markdown links
        pattern = r'\[([^\]]+)\]\(commands/([^\)]+)\)'
        
        def replace_link(match):
            link_text = match.group(1)
            url = match.group(2)
            
            # Convert commands/xxx.md to just xxx.md (relative path)
            return f"[{link_text}]({url})"
        
        new_content = re.sub(pattern, replace_link, content)
        
        if new_content != original_content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)
            return True
        return False
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def main():
    docs_dir = Path("docs")
    
    if not docs_dir.exists():
        print("Error: docs directory not found")
        return
    
    # Find all markdown files in commands directory
    commands_dir = docs_dir / "commands"
    if not commands_dir.exists():
        print("Error: commands directory not found")
        return
    
    md_files = list(commands_dir.glob("*.md"))
    
    print(f"Found {len(md_files)} markdown files in commands directory")
    print("\nFixing relative links...")
    
    fixed_count = 0
    for md_file in md_files:
        if fix_links_in_file(md_file):
            try:
                rel_path = md_file.relative_to(Path.cwd())
            except ValueError:
                rel_path = md_file
            print(f"  Fixed: {rel_path}")
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} files")

if __name__ == "__main__":
    main()

