import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons

Item {
  id: root

  property var pluginApi: null

  // Logging helpers with fallback
  function log(level, msg) {
    var prefix = "[NiriWriter] ";
    if (typeof Logger !== 'undefined') {
      if (level === "e") Logger.e("NiriWriter", msg);
      else if (level === "w") Logger.w("NiriWriter", msg);
      else if (level === "i") Logger.i("NiriWriter", msg);
      else Logger.d("NiriWriter", msg);
    }
    console.log(prefix + msg);
  }

  signal saveComplete(bool success, string message)

  property var pendingChanges: []
  property int currentChangeIndex: 0
  property var currentEditorData: null

  function saveAll(editorData) {
    log("i", "=== saveAll called ===");
    console.log("[NiriWriter] saveAll called, editorData:", JSON.stringify(editorData ? {cats: editorData.categories?.length, del: editorData.deletedBinds?.length} : null));
    if (!editorData || !editorData.categories) {
      log("e", "No editor data provided");
      saveComplete(false, "No editor data provided");
      return;
    }

    log("i", "Categories: " + editorData.categories.length);
    log("i", "Deleted binds: " + (editorData.deletedBinds ? editorData.deletedBinds.length : 0));

    currentEditorData = editorData;
    pendingChanges = [];
    currentChangeIndex = 0;

    // Collect all affected files for backup
    var affectedFiles = collectAffectedFiles(editorData);
    log("i", "Affected files: " + JSON.stringify(affectedFiles));

    // Collect all changes
    for (var i = 0; i < editorData.categories.length; i++) {
      var cat = editorData.categories[i];
      log("d", "Checking category: " + cat.title + " (titleChanged: " + cat.titleChanged + ", binds: " + (cat.binds ? cat.binds.length : 0) + ")");

      // Category rename is not supported for Niri (categories are auto-detected from actions)
      // Would need manual category markers in the config file to work
      if (cat.titleChanged && cat.originalTitle) {
        log("w", "Category rename not supported for Niri (auto-detected categories)");
      }

      for (var j = 0; j < cat.binds.length; j++) {
        var bind = cat.binds[j];
        log("d", "  Bind: " + bind.keys + " status=" + bind.status + " sourceFile=" + bind.sourceFile);

        if (bind.status === "modified") {
          pendingChanges.push({
            type: "modify",
            bind: bind,
            category: cat.title
          });
        } else if (bind.status === "new") {
          pendingChanges.push({
            type: "add",
            bind: bind,
            category: cat.title
          });
        }
      }
    }

    // Add deletions
    log("i", "Deleted binds to process: " + (editorData.deletedBinds ? editorData.deletedBinds.length : 0));
    for (var k = 0; k < editorData.deletedBinds.length; k++) {
      var delBind = editorData.deletedBinds[k];
      log("d", "  Delete bind: " + delBind.keys + " sourceFile=" + delBind.sourceFile + " rawLine=" + (delBind.rawLine ? "yes" : "no"));
      pendingChanges.push({
        type: "delete",
        bind: delBind
      });
    }

    log("i", "Pending changes collected: " + pendingChanges.length);
    for (var pc = 0; pc < pendingChanges.length; pc++) {
      log("d", "  Change " + pc + ": " + pendingChanges[pc].type);
    }

    if (pendingChanges.length === 0) {
      log("w", "No changes to save - completing");
      saveComplete(true, "No changes to save");
      return;
    }

    // Create backups first
    log("i", "Creating backups...");
    createBackups(affectedFiles);
  }

  function collectAffectedFiles(editorData) {
    var files = {};

    for (var i = 0; i < editorData.categories.length; i++) {
      var cat = editorData.categories[i];

      // Include files from category rename
      if (cat.titleChanged && cat.binds && cat.binds.length > 0 && cat.binds[0].sourceFile) {
        files[cat.binds[0].sourceFile] = true;
      }

      for (var j = 0; j < cat.binds.length; j++) {
        var bind = cat.binds[j];
        if (bind.sourceFile && bind.sourceFile !== "") {
          files[bind.sourceFile] = true;
        }
      }
    }

    for (var k = 0; k < editorData.deletedBinds.length; k++) {
      var delBind = editorData.deletedBinds[k];
      if (delBind.sourceFile) {
        files[delBind.sourceFile] = true;
      }
    }

    return Object.keys(files);
  }

  function createBackups(files) {
    log("i", "createBackups called, files: " + files.length);
    if (files.length === 0) {
      log("w", "No files to backup, skipping to changes");
      processNextChange();
      return;
    }

    // Single backup file per config (overwrites previous backup)
    var commands = files.map(function(f) {
      var backupPath = f + ".backup";
      return "cp '" + escapeShell(f) + "' '" + escapeShell(backupPath) + "'";
    });

    var cmd = commands.join(" && ");
    log("i", "Backup command: " + cmd);
    backupProcess.command = ["sh", "-c", cmd];
    backupProcess.running = true;
  }

  Process {
    id: backupProcess
    running: false

    onRunningChanged: {
      if (running) {
        log("i", "backupProcess STARTED: " + JSON.stringify(command));
      }
    }

    onExited: function(exitCode) {
      log("i", "backupProcess exited with code: " + exitCode);
      if (exitCode === 0) {
        log("i", "Backups created successfully, processing changes...");
        root.processNextChange();
      } else {
        log("e", "Backup FAILED with code: " + exitCode);
        root.saveComplete(false, "Failed to create backups");
      }
    }

    stderr: SplitParser {
      onRead: data => log("e", "Backup stderr: " + data)
    }
  }

  function processNextChange() {
    log("i", "processNextChange: " + currentChangeIndex + "/" + pendingChanges.length);
    if (currentChangeIndex >= pendingChanges.length) {
      log("i", "=== All changes processed, reloading niri config ===");
      reloadProcess.running = true;
      // saveComplete will be called after reload finishes
      return;
    }

    var change = pendingChanges[currentChangeIndex];
    currentChangeIndex++;

    log("i", "Processing change type: " + change.type);

    switch (change.type) {
      case "modify":
        modifyBind(change.bind);
        break;
      case "add":
        addBind(change.bind, change.category);
        break;
      case "delete":
        deleteBind(change.bind);
        break;
      case "renameCategory":
        renameCategoryInFile(change.category, change.sourceFile);
        break;
      default:
        log("w", "Unknown change type: " + change.type);
        processNextChange();
    }
  }

  function modifyBind(bind) {
    if (!bind.sourceFile || !bind.rawLine) {
      log("w", "Cannot modify bind without source info");
      processNextChange();
      return;
    }

    var newLine = formatNiriBind(bind);

    // Extract indentation from original line
    var indentMatch = bind.rawLine.match(/^(\s*)/);
    var indent = indentMatch ? indentMatch[1] : "    ";

    // Use the trimmed content for matching but preserve structure
    var oldContent = bind.rawLine.trim();
    // Pattern (left side) needs full escaping including $
    var oldLineEsc = escapeForSedPattern(oldContent);
    // Replacement (right side) should NOT escape $ (literal in sed replacement)
    var newLineEsc = escapeForSedReplacement(newLine);

    log("d", "Modifying bind:");
    log("d", "  Old: " + oldContent);
    log("d", "  New: " + newLine);
    log("d", "  Old escaped: " + oldLineEsc);
    log("d", "  New escaped: " + newLineEsc);

    // Match line with any leading whitespace, replace preserving indentation
    var cmd = "sed -i 's|^\\(\\s*\\)" + oldLineEsc + "$|\\1" + newLineEsc + "|' '" + escapeShell(bind.sourceFile) + "'";

    log("d", "Command: " + cmd);

    writeProcess.command = ["sh", "-c", cmd];
    writeProcess.running = true;
  }

  function addBind(bind, category) {
    // Determine target file
    var homeDir = Quickshell.env("HOME");
    var targetFile = pluginApi?.pluginSettings?.niriConfigPath || (homeDir + "/.config/niri/config.kdl");
    targetFile = targetFile.replace(/^~/, homeDir);

    var newLine = formatNiriBind(bind);

    // Try to find category marker or end of binds block
    // For Niri, we'll insert at the end of the binds block
    var categoryMarker = '// #"' + category + '"';
    var escapedMarker = escapeForSed(categoryMarker);

    // Check if category marker exists
    findCategoryProcess.targetFile = targetFile;
    findCategoryProcess.newLine = newLine;
    findCategoryProcess.categoryMarker = categoryMarker;
    findCategoryProcess.command = ["sh", "-c", "grep -n '" + escapedMarker + "' '" + escapeShell(targetFile) + "' | head -1 | cut -d: -f1"];
    findCategoryProcess.running = true;
  }

  Process {
    id: findCategoryProcess
    property string targetFile: ""
    property string newLine: ""
    property string categoryMarker: ""
    property string lineNumber: ""
    running: false

    stdout: SplitParser {
      onRead: data => findCategoryProcess.lineNumber = data.trim()
    }

    onExited: function(exitCode) {
      root.log("d", "Category search result: line=" + lineNumber + ", exitCode=" + exitCode);
      root.log("d", "newLine to write: " + newLine);

      // Escape single quotes for printf - this is the ONLY escaping needed
      // Inside single quotes: special chars stay literal
      var safeLine = "    " + newLine.replace(/'/g, "'\\''");
      root.log("d", "safeLine after quote escape: " + safeLine);

      var cmd;

      if (lineNumber && lineNumber !== "") {
        // Insert after category marker using temp file + sed 'r' command
        // This avoids sed's a\ command which can interpret backslash sequences
        root.log("d", "Found category at line " + lineNumber + ", inserting after");
        var tempFile = "/tmp/niri_bind_" + Date.now();
        cmd = "printf '%s\\n' '" + safeLine + "' > '" + tempFile + "' && " +
              "sed -i '" + lineNumber + "r " + tempFile + "' '" + root.escapeShell(targetFile) + "' && " +
              "rm -f '" + tempFile + "'";
      } else {
        // Find end of binds block and insert before closing brace using awk
        root.log("d", "Category not found, inserting into binds block");
        cmd = "awk '/^binds \\{/,/^\\}/ { if (/^\\}/) print \"" + root.escapeForAwk(safeLine) + "\"; print; next } 1' '" + root.escapeShell(targetFile) + "' > '" + root.escapeShell(targetFile) + ".tmp' && mv '" + root.escapeShell(targetFile) + ".tmp' '" + root.escapeShell(targetFile) + "'";
      }

      root.log("d", "Insert command: " + cmd);
      lineNumber = "";
      writeProcess.command = ["sh", "-c", cmd];
      writeProcess.running = true;
    }
  }

  function deleteBind(bind) {
    if (!bind.sourceFile || !bind.rawLine) {
      log("w", "Cannot delete bind without source info");
      processNextChange();
      return;
    }

    var oldContent = bind.rawLine.trim();
    // Pattern needs full escaping for matching
    var patternLine = escapeForSedPattern(oldContent);

    log("d", "Deleting bind: " + oldContent);

    // Actually delete the line (backups are created before saving)
    // Match line with any leading whitespace
    // Use \| as delimiter to avoid conflicts with / in paths
    var cmd = "sed -i '\\|^[[:space:]]*" + patternLine + "$|d' '" + escapeShell(bind.sourceFile) + "'";

    writeProcess.command = ["sh", "-c", cmd];
    writeProcess.running = true;
  }

  function renameCategoryInFile(category, sourceFile) {
    // Only rename if the title was actually changed
    if (!category.titleChanged || !category.originalTitle) {
      log("d", "Category not changed, skipping: " + category.title);
      processNextChange();
      return;
    }

    var oldTitle = category.originalTitle;
    var newTitle = category.title;

    log("i", "Renaming category '" + oldTitle + "' to '" + newTitle + "' in " + sourceFile);

    // Get the first bind to find where to insert the marker
    if (!category.binds || category.binds.length === 0 || !category.binds[0].rawKeys) {
      log("w", "Category has no binds with rawKeys, cannot add marker");
      processNextChange();
      return;
    }

    var firstBind = category.binds[0];
    // Use rawKeys (e.g., "Mod+Q") to find the line - simpler and more reliable
    var keyPattern = firstBind.rawKeys;

    // Markers can be either // #"Name" or // #Name
    var oldPatternQuoted = '// #"' + oldTitle + '"';
    var oldPatternSimple = '// #' + oldTitle;
    var newMarker = '// #"' + newTitle + '"';

    log("d", "Looking for bind with keys: " + keyPattern);

    // Command:
    // 1. Try to replace existing quoted marker // #"Name"
    // 2. If not found, try simple marker // #Name
    // 3. If no marker, find line number of first bind (by key pattern) and insert marker before it
    var cmd = "if grep -q '// #\"" + oldTitle + "\"' '" + escapeShell(sourceFile) + "'; then " +
              "  sed -i 's|// #\"" + oldTitle + "\"|" + newMarker + "|g' '" + escapeShell(sourceFile) + "'; " +
              "  echo 'Updated quoted marker'; " +
              "elif grep -q '// #" + oldTitle + "' '" + escapeShell(sourceFile) + "'; then " +
              "  sed -i 's|// #" + oldTitle + "|" + newMarker + "|g' '" + escapeShell(sourceFile) + "'; " +
              "  echo 'Updated simple marker'; " +
              "else " +
              "  LINE=$(grep -n '^[[:space:]]*" + keyPattern + " ' '" + escapeShell(sourceFile) + "' | head -1 | cut -d: -f1); " +
              "  echo \"Found bind at line: $LINE\"; " +
              "  if [ -n \"$LINE\" ]; then " +
              "    sed -i \"${LINE}i\\    " + newMarker + "\" '" + escapeShell(sourceFile) + "'; " +
              "    echo 'Inserted new marker'; " +
              "  else " +
              "    echo 'Could not find bind'; " +
              "  fi; " +
              "fi";

    log("d", "Rename/insert command: " + cmd);

    writeProcess.command = ["sh", "-c", cmd];
    writeProcess.running = true;
  }

  Process {
    id: writeProcess
    running: false

    onRunningChanged: {
      if (running) {
        log("i", "writeProcess STARTED: " + JSON.stringify(command));
      }
    }

    onExited: function(exitCode) {
      log("i", "writeProcess exited with code: " + exitCode);
      if (exitCode !== 0) {
        log("e", "Write FAILED with exit code: " + exitCode);
      }
      root.processNextChange();
    }

    stdout: SplitParser {
      onRead: data => log("d", "Write stdout: " + data)
    }

    stderr: SplitParser {
      onRead: data => log("e", "Write stderr: " + data)
    }
  }

  // Reload niri config after all changes are saved
  Process {
    id: reloadProcess
    running: false
    command: ["niri", "msg", "action", "load-config-file"]

    onExited: function(exitCode) {
      if (exitCode === 0) {
        log("i", "Niri config reloaded successfully");
        root.saveComplete(true, "Saved and reloaded!");
      } else {
        log("e", "Niri config reload FAILED (code: " + exitCode + ") - check config for errors!");
        root.saveComplete(false, "Saved but reload failed - check config syntax!");
      }
    }

    stderr: SplitParser {
      onRead: data => log("e", "Niri reload stderr: " + data)
    }
  }

  function formatNiriBind(bind) {
    // Format: Mod+Key hotkey-overlay-title="description" { action; }
    var keyCombo = formatNiriKeyCombo(bind);
    var description = bind.description || "";
    var action = bind.action || "spawn \"\"";

    var line = keyCombo;
    if (description !== "") {
      line += ' hotkey-overlay-title="' + description + '"';
    }
    line += " { " + action + "; }";

    return line;
  }

  function formatNiriKeyCombo(bind) {
    var parts = [];

    if (bind.modifiers) {
      for (var i = 0; i < bind.modifiers.length; i++) {
        var mod = bind.modifiers[i];
        if (mod === "Super") parts.push("Mod");
        else if (mod === "Ctrl") parts.push("Ctrl");
        else if (mod === "Shift") parts.push("Shift");
        else if (mod === "Alt") parts.push("Alt");
      }
    }

    if (bind.key) {
      parts.push(bind.key);
    }

    return parts.join("+");
  }

  function escapeForSedPattern(str) {
    // Escape special characters for sed PATTERN (left side of s|||)
    // In basic sed: {} + ? ( ) are LITERAL, not special
    // Only these are special in pattern: \ | & ^ $ . * [ ]
    return str
      .replace(/\\/g, '\\\\')      // Backslash first
      .replace(/\|/g, '\\|')       // Pipe (our delimiter)
      .replace(/&/g, '\\&')        // Ampersand
      .replace(/\^/g, '\\^')       // Caret (start anchor)
      .replace(/\$/g, '\\$')       // Dollar (end anchor)
      .replace(/\./g, '\\.')       // Dot (any char)
      .replace(/\*/g, '\\*')       // Asterisk (zero or more)
      .replace(/\[/g, '\\[')       // Brackets (char class)
      .replace(/\]/g, '\\]')
      .replace(/\n/g, '')          // Remove newlines
      .replace(/\t/g, ' ')         // Convert tabs to spaces
      .replace(/\r/g, '');         // Remove carriage returns
  }

  function escapeForSedReplacement(str) {
    // Escape special characters for sed REPLACEMENT (right side of s|||)
    // In replacement: only \ | & are special
    // $ is NOT special unless followed by digit (backreference)
    return str
      .replace(/\\/g, '\\\\')      // Backslash first
      .replace(/\|/g, '\\|')       // Pipe (our delimiter)
      .replace(/&/g, '\\&')        // Ampersand (inserts matched text)
      .replace(/\n/g, '')          // Remove newlines
      .replace(/\t/g, ' ')         // Convert tabs to spaces
      .replace(/\r/g, '');         // Remove carriage returns
  }

  // Keep old name for backward compatibility (uses pattern escaping)
  function escapeForSed(str) {
    return escapeForSedPattern(str);
  }

  function escapeForAwk(str) {
    // Escape special characters for awk
    return str
      .replace(/\\/g, '\\\\')      // Backslash first
      .replace(/'/g, "\\'")        // Single quotes
      .replace(/"/g, '\\"')        // Double quotes
      .replace(/\n/g, '\\n')       // Newline
      .replace(/\t/g, '\\t');      // Tab
  }

  function escapeShell(str) {
    // Comprehensive shell escaping for safe command execution
    return str
      .replace(/\\/g, '\\\\')      // Backslash first
      .replace(/'/g, "'\\''")      // Single quotes
      .replace(/"/g, '\\"')        // Double quotes
      .replace(/`/g, '\\`')        // Backticks (command substitution)
      .replace(/\$/g, '\\$')       // Dollar (variable expansion)
      .replace(/!/g, '\\!')        // History expansion
      .replace(/\n/g, '\\n')       // Newline
      .replace(/\t/g, '\\t');      // Tab
  }

  function sanitizeDescription(str) {
    // Remove any potentially dangerous characters from user descriptions
    if (!str) return "";
    return str.replace(/[`$\\"\n\r\t]/g, '');
  }
}
