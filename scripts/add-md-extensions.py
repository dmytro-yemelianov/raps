#!/usr/bin/env python3
"""
Add .md extensions to relative links in Markdown files for MkDocs compatibility.
"""

import re
from pathlib import Path

def fix_links_in_file(file_path):
    """Add .md extensions to relative links."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        original_content = content
        
        # Pattern to match markdown links: [text](path) or [text](path#anchor)
        # Exclude: http/https URLs, anchors starting with #, mailto:, and already .md extensions
        pattern = r'\[([^\]]+)\]\(([^\)]+)\)'
        
        def replace_link(match):
            link_text = match.group(1)
            url = match.group(2)
            
            # Skip if it's already a full URL, mailto, or starts with #
            if url.startswith('http://') or url.startswith('https://') or \
               url.startswith('mailto:') or url.startswith('#') or \
               url.startswith('ftp://') or url.startswith('file://'):
                return match.group(0)
            
            # Skip if it already has .md extension
            if url.endswith('.md') or url.endswith('.md#'):
                return match.group(0)
            
            # Check if it's a relative path (not absolute)
            # Add .md before # anchor if present
            if '#' in url:
                path, anchor = url.split('#', 1)
                # Skip if path is empty (just anchor)
                if not path:
                    return match.group(0)
                # Skip if it's a special file like SECURITY, RELEASE (these might not exist as .md)
                if path.upper() in ['SECURITY', 'RELEASE']:
                    return match.group(0)
                # Add .md before the anchor
                new_url = f"{path}.md#{anchor}"
            else:
                # Skip if it's a special file
                if url.upper() in ['SECURITY', 'RELEASE']:
                    return match.group(0)
                new_url = f"{url}.md"
            
            return f"[{link_text}]({new_url})"
        
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
    
    # Find all markdown files
    md_files = list(docs_dir.rglob("*.md"))
    
    print(f"Found {len(md_files)} markdown files")
    print("\nAdding .md extensions to relative links...")
    
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

