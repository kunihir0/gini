#!/usr/bin/env bun
/**
 * Copilot Extension Modifier
 * 
 * This script modifies GitHub Copilot Chat extension files to enable various features.
 * It automatically finds and processes extension installations in standard locations.
 * Dependency-free implementation with native TypeScript and Bun support.
 */

import { existsSync, statSync, readdirSync, readFileSync, writeFileSync, copyFileSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';

// Type definitions
type ExtensionInfo = {
  path: string;
  modified: boolean;
};

type ModificationResult = {
  wasModified: boolean;
  backupPath: string | null;
};

// Simple ANSI color codes for terminal output
const COLORS = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  red: '\x1b[31m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  cyan: '\x1b[36m'
};

/**
 * Get the root paths for VSCode/editor extensions based on the user's home directory.
 */
const getRootPaths = (customPath?: string): string[] => {
  const homeDirectory = homedir();
  const validPaths: string[] = [];

  if (customPath) {
    validPaths.push(customPath);
    console.log(`${COLORS.blue}Using custom path: ${COLORS.bold}${customPath}${COLORS.reset}`);
  } else {
    const potentialPaths = [
      join(homeDirectory, '.vscode', 'extensions'),
      join(homeDirectory, '.vscode-server', 'extensions'),
      join(homeDirectory, '.vscode-insiders', 'extensions'),
      join(homeDirectory, '.vscode-server-insiders', 'extensions'),
      join(homeDirectory, '.cursor', 'extensions'),
      join(homeDirectory, '.cursor-server', 'extensions'),
      join(homeDirectory, '.vscode-oss', 'extensions'),
      join(homeDirectory, '.vscode-oss-dev', 'extensions'),
    ];

    potentialPaths.forEach(path => {
      if (existsSync(path)) {
        validPaths.push(path);
      }
    });
    
    console.log(`${COLORS.blue}Found valid extension paths:${COLORS.reset}`);
    validPaths.forEach(path => console.log(`${COLORS.dim}  → ${path}${COLORS.reset}`));
  }
  
  console.log(''); // Empty line for spacing
  return validPaths;
};

/**
 * Create a backup of the target file
 */
const createBackup = (filePath: string): string | null => {
  try {
    let backupFilePath = `${filePath}.bak`;
    
    // If backup already exists, add timestamp to make unique
    if (existsSync(backupFilePath)) {
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
      backupFilePath += `_${timestamp}`;
    }
    
    copyFileSync(filePath, backupFilePath);
    return backupFilePath;
  } catch (error) {
    console.error(`${COLORS.red}Failed to create backup for ${filePath}: ${error}${COLORS.reset}`);
    return null;
  }
};

/**
 * Modify the content of the extension file
 */
const modifyFileContent = (content: string): string => {
  // Modifications to apply (in order)
  const modifications = [
    {
      description: "Enable all models for LM API usage",
      pattern: /,"x-onbehalf-extension-id":`\$\{[a-zA-Z]+\}\/\$\{[a-zA-Z]+\}`/g,
      replacement: '',
    },
    {
      description: "Enable BYOK feature for Business and Enterprise",
      pattern: /get isIndividual\(\){return this._info\.individual\?\?!1}/g,
      replacement: 'get isIndividual(){return !0;this._info.individual??!1}',
    },
    {
      description: "Enable CodeReview",
      pattern: /get isCopilotCodeReviewEnabled\(\){return this\.getTokenValue\("ccr"\)==="1"}/g,
      replacement: 'get isCopilotCodeReviewEnabled(){return !0}',
    },
  ];

  let modifiedContent = content;
  let wasChanged = false;

  for (const mod of modifications) {
    const beforeLength = modifiedContent.length;
    modifiedContent = modifiedContent.replace(mod.pattern, mod.replacement);
    
    if (modifiedContent.length !== beforeLength) {
      wasChanged = true;
    }
  }

  return modifiedContent;
};

/**
 * Update a specific file with our modifications
 */
const updateFile = (filePath: string): ModificationResult => {
  try {
    if (!existsSync(filePath)) {
      console.log(`${COLORS.yellow}File doesn't exist: ${filePath}${COLORS.reset}`);
      return { wasModified: false, backupPath: null };
    }
    
    const content = readFileSync(filePath, 'utf-8');
    const originalContentLength = content.length;
    
    // Create backup
    const backupPath = createBackup(filePath);
    if (backupPath) {
      console.log(`${COLORS.dim}Backup created: ${backupPath}${COLORS.reset}`);
    }
    
    // Modify content
    const modifiedContent = modifyFileContent(content);
    
    // Check if content changed
    const wasModified = originalContentLength !== modifiedContent.length;
    
    // Write changes back to file
    writeFileSync(filePath, modifiedContent, 'utf-8');
    
    if (wasModified) {
      console.log(`${COLORS.green}✓ File updated: ${filePath}${COLORS.reset}`);
    } else {
      console.log(`${COLORS.dim}○ No changes needed: ${filePath}${COLORS.reset}`);
    }
    
    return { wasModified, backupPath };
    
  } catch (error) {
    console.error(`${COLORS.red}Error updating ${filePath}: ${error}${COLORS.reset}`);
    return { wasModified: false, backupPath: null };
  }
};

/**
 * Process all the Copilot Chat extensions in a directory
 */
const processCopilotExtensions = (directoryPath: string): number => {
  // Check if the directory exists
  if (!existsSync(directoryPath) || !statSync(directoryPath).isDirectory()) {
    console.log(`${COLORS.yellow}Directory not found: ${directoryPath}${COLORS.reset}`);
    console.log('');
    return 0;
  }
  
  // Process extensions
  let modifiedCount = 0;
  const versionPattern = /^github\.copilot-chat-\d+\.\d+\.\d+$/;
  
  readdirSync(directoryPath).forEach(fileName => {
    const fullPath = join(directoryPath, fileName);
    
    // Check if it's a directory and matches the Copilot Chat extension pattern
    if (
      statSync(fullPath).isDirectory() && 
      fileName.startsWith('github.copilot-chat-') && 
      versionPattern.test(fileName)
    ) {
      console.log(`${COLORS.blue}Processing Copilot Chat extension: ${COLORS.bold}${fileName}${COLORS.reset}`);
      
      // Target file path
      const extensionFile = join(fullPath, 'dist', 'extension.js');
      
      // Update the file
      const result = updateFile(extensionFile);
      if (result.wasModified) {
        modifiedCount++;
      }
    }
  });
  
  console.log(`${COLORS.blue}\nModified ${modifiedCount} extension files in ${directoryPath}${COLORS.reset}`);
  console.log(''); // Empty line for spacing
  
  return modifiedCount;
};

/**
 * Main execution
 */
const main = (): void => {
  console.log(`${COLORS.bold}${COLORS.blue}\n🧙 Copilot Extension Modifier\n${COLORS.reset}`);
  
  // Get command line argument for custom path
  const customPath = Bun.argv[2];
  
  // Get extension root paths
  const rootPaths = getRootPaths(customPath);
  
  if (rootPaths.length === 0) {
    console.log(`${COLORS.yellow}No extension paths found. Try providing a custom path as an argument.${COLORS.reset}`);
    return;
  }
  
  // Process all found paths
  let totalModified = 0;
  for (const rootPath of rootPaths) {
    totalModified += processCopilotExtensions(rootPath);
  }
  
  // Final summary
  if (totalModified > 0) {
    console.log(`${COLORS.green}${COLORS.bold}✅ Successfully modified ${totalModified} extension files.${COLORS.reset}`);
  } else {
    console.log(`${COLORS.yellow}No extensions were modified. They might be already modified or not present.${COLORS.reset}`);
  }
};

// Execute
main();
