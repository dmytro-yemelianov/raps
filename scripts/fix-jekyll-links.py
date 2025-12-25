#!/usr/bin/env python3
"""
Convert Jekyll-style relative_url links to standard Markdown links for MkDocs.
"""

import re
import os
from pathlib import Path

def convert_jekyll_link(match):
    """Convert Jekyll link to Markdown link."""
    link_text = match.group(1)
    url_part = match.group(2)
    
    # Remove leading slash and convert to relative path
    # MkDocs doesn't need .md extension
    url = url_part.lstrip('/')
    
    return f"[{link_text}]({url})"

def fix_file(file_path):
    """Fix Jekyll links in a single file."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Pattern to match: [text]({{ '/path' | relative_url }})
        # or [text]({{ '/path' | relative_url }}#anchor)
        pattern = r'\[([^\]]+)\]\(\{\{\s*[\'"]?([^\'"]+)[\'"]?\s*\|\s*relative_url\s*\}\}(?:#([^\]]+))?\)'
        
        def replace_link(match):
            link_text = match.group(1)
            url_part = match.group(2)
            anchor = match.group(3) if match.lastindex >= 3 else None
            
            # Remove leading slash
            url = url_part.lstrip('/')
            
            # Add .md extension if not already present (MkDocs requires it)
            if not url.endswith('.md') and not url.startswith('http') and not url.startswith('#'):
                url = f"{url}.md"
            
            # Add anchor if present
            if anchor:
                url = f"{url}#{anchor}"
            
            return f"[{link_text}]({url})"
        
        new_content = re.sub(pattern, replace_link, content)
        
        if new_content != content:
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
    
    # Find all markdown files
    md_files = list(docs_dir.rglob("*.md"))
    
    print(f"Found {len(md_files)} markdown files")
    print("\nConverting Jekyll links to Markdown...")
    
    fixed_count = 0
    for md_file in md_files:
        if fix_file(md_file):
            try:
                rel_path = md_file.relative_to(Path.cwd())
            except ValueError:
                rel_path = md_file
            print(f"  Fixed: {rel_path}")
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} files")

if __name__ == "__main__":
    main()

