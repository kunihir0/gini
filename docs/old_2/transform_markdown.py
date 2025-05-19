import re
import sys
import os

def transform_content(content):
    lines = content.splitlines()
    transformed_lines = []
    in_key_findings = False
    in_actionable_recommendations = False
    
    # First pass for H2 and H3 transformations, and simple line cleaning
    temp_lines = []
    for i, line in enumerate(lines):
        stripped_line = line.strip()
        
        if stripped_line.startswith("File Path: "): # Handles example and general case
             path_content = stripped_line[len('File Path: '):].strip()
             temp_lines.append(f"## File Path: `{path_content}`")
             continue

        if stripped_line == "Overall Assessment:":
            temp_lines.append("## Overall Assessment")
            in_key_findings = False
            in_actionable_recommendations = False
            continue
        if stripped_line == "Key Findings and Suggestions:":
            temp_lines.append("## Key Findings and Suggestions")
            in_key_findings = True
            in_actionable_recommendations = False
            continue
        if stripped_line == "Actionable Recommendations:":
            temp_lines.append("## Actionable Recommendations")
            in_key_findings = False
            in_actionable_recommendations = True
            continue

        if in_key_findings:
            match_2c = re.match(r"^\*\s+(.+?):$", stripped_line)
            if match_2c:
                heading_text = match_2c.group(1).strip()
                temp_lines.append(f"### {heading_text}")
                continue

        temp_lines.append(line) 

    processed_lines = []
    current_h3_active = False 

    for i, line_content in enumerate(temp_lines):
        original_line_stripped = line_content.strip() 
        
        if original_line_stripped == "## Key Findings and Suggestions":
            in_key_findings = True
            in_actionable_recommendations = False
            processed_lines.append(original_line_stripped)
            current_h3_active = False
            continue
        elif original_line_stripped == "## Actionable Recommendations":
            in_key_findings = False
            in_actionable_recommendations = True
            processed_lines.append(original_line_stripped)
            current_h3_active = False
            continue
        elif original_line_stripped.startswith("## "): # Catches "## File Path: `...`" and "## Overall Assessment"
            in_key_findings = False # Reset state if it's any H2
            in_actionable_recommendations = False # Reset state
            processed_lines.append(original_line_stripped)
            current_h3_active = False
            continue
        
        if original_line_stripped.startswith("### "):
            current_h3_active = True # This H3 is under "Key Findings" due to prior state check
            processed_lines.append(original_line_stripped)
            continue

        if in_key_findings and current_h3_active:
            match_2d = re.match(r"^\s{2,}\*\s+(.*)", line_content) 
            if match_2d:
                item_text = match_2d.group(1).strip()
                processed_lines.append(f"- {item_text}")
                continue
        
        if in_actionable_recommendations:
            if re.match(r"^\d+\.\s+.*", original_line_stripped):
                processed_lines.append(original_line_stripped) 
                continue
            
            sub_item_match = re.match(r"^(\s*)(\*|-)\s+(.*)", line_content)
            if sub_item_match:
                leading_spaces = sub_item_match.group(1)
                marker = sub_item_match.group(2)
                rest = sub_item_match.group(3).strip()
                if len(leading_spaces) >= 1: 
                    if len(leading_spaces) >= 8: 
                        processed_lines.append(f"{leading_spaces}{marker} {rest}")
                    else: 
                        processed_lines.append(f"    {marker} {rest}")
                    continue
        
        if original_line_stripped:
            processed_lines.append(original_line_stripped)
        elif not processed_lines or processed_lines[-1].strip() != "":
            processed_lines.append("")


    deduped_blank_lines = []
    if processed_lines and processed_lines[0].strip() == "": # Remove leading blank line if any from processing
        start_index = 1
        while start_index < len(processed_lines) and processed_lines[start_index].strip() == "":
            start_index += 1
        processed_lines = processed_lines[start_index:]

    for i, l in enumerate(processed_lines):
        if l.strip() == "" and deduped_blank_lines and deduped_blank_lines[-1].strip() == "":
            continue
        deduped_blank_lines.append(l)
        
    final_text = "\n".join(deduped_blank_lines)
    final_text = final_text.strip() # Remove leading/trailing newlines from the whole content
    final_text = re.sub(r'\n{3,}', '\n\n', final_text) # Consolidate multiple blank lines
    
    return final_text


def check_if_transformed(file_path):
    """Checks if the file seems to be already transformed."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            for line in f:
                stripped_line = line.strip()
                if not stripped_line: # Skip empty lines
                    continue
                # If the first non-empty line matches the transformed file path pattern
                if stripped_line.startswith("## File Path: `"):
                    return True
                return False # First non-empty line is not the transformed pattern
    except Exception:
        return False # If error reading, assume not transformed
    return False


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python transform_markdown.py <file_path1> [file_path2 ...]")
        sys.exit(1)

    files_processed_count = 0
    files_skipped_count = 0

    for file_path in sys.argv[1:]:
        if not file_path.endswith(".md"):
            print(f"Skipping non-markdown file: {file_path}")
            files_skipped_count +=1
            continue

        if not os.path.exists(file_path):
            print(f"Error: File not found at {file_path}")
            continue

        if check_if_transformed(file_path):
            print(f"Skipping already transformed file: {file_path}")
            files_skipped_count += 1
            continue

        print(f"Processing file: {file_path}")
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                original_content = f.read()
        except Exception as e:
            print(f"Error reading file {file_path}: {e}")
            continue

        transformed_content = transform_content(original_content)

        try:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(transformed_content)
            files_processed_count += 1
        except Exception as e:
            print(f"Error writing to file {file_path}: {e}")
            continue
    
    print(f"\nTransformation complete.")
    print(f"Files processed: {files_processed_count}")
    print(f"Files skipped (already transformed or non-markdown): {files_skipped_count}")