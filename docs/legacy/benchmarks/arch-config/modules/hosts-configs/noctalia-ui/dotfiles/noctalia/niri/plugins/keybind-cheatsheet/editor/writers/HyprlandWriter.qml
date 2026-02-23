import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons

Item {
  id: root

  property var pluginApi: null

  // Logging helper
  function log(level, msg) {
    var prefix = "[HyprlandWriter] ";
    if (typeof Logger !== 'undefined') {
      if (level === "e") Logger.e("HyprlandWriter", msg);
      else if (level === "w") Logger.w("HyprlandWriter", msg);
      else if (level === "i") Logger.i("HyprlandWriter", msg);
      else Logger.d("HyprlandWriter", msg);
    }
    console.log(prefix + msg);
  }

  signal saveComplete(bool success, string message)

  property var pendingChanges: []
  property int currentChangeIndex: 0
  property var currentEditorData: null
  property var originalCategoryTitles: ({})  // Track original titles for rename detection

  function saveAll(editorData) {
    log("i", "=== saveAll called ===");
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

    // First, create backups of all affected files
    var affectedFiles = collectAffectedFiles(editorData);
    log("i", "Affected files: " + JSON.stringify(affectedFiles));

    // Collect all changes in correct order:
    // 1. Deletions first (so moved binds are removed before being re-added)
    // 2. Then category operations
    // 3. Then additions

    var deleteChanges = [];
    var categoryChanges = [];
    var addChanges = [];

    // Collect bind deletions FIRST
    log("i", "Processing deletedBinds: " + editorData.deletedBinds.length);
    for (var db = 0; db < editorData.deletedBinds.length; db++) {
      var delBind = editorData.deletedBinds[db];
      log("d", "  Bind " + db + ": keys=" + delBind.keys + " sourceFile=" + delBind.sourceFile + " rawLine=" + (delBind.rawLine ? "yes" : "no"));
      if (delBind.sourceFile && delBind.rawLine) {
        deleteChanges.push({
          type: "delete",
          bind: delBind
        });
      } else {
        log("w", "  Skipping bind without source info: " + delBind.keys);
      }
    }

    // Collect category header deletions
    if (editorData.deletedCategories) {
      for (var dc = 0; dc < editorData.deletedCategories.length; dc++) {
        var delCat = editorData.deletedCategories[dc];
        log("i", "Category to delete: " + delCat.title + " in " + delCat.sourceFile);
        deleteChanges.push({
          type: "deleteCategoryHeader",
          category: delCat
        });
      }
    }

    // Collect category and bind changes
    for (var i = 0; i < editorData.categories.length; i++) {
      var cat = editorData.categories[i];

      // Check for category rename - use cat.sourceFile instead of bind sourceFile
      if (cat.titleChanged && cat.originalTitle && cat.sourceFile) {
        categoryChanges.push({
          type: "renameCategory",
          category: cat,
          sourceFile: cat.sourceFile
        });
      }

      // Check if this is a new category by looking at cat.sourceFile
      // If cat.sourceFile exists, category header already exists in file
      // If not, we need to create the header
      if (!cat.sourceFile) {
        log("i", "New category detected: " + cat.title + " (no sourceFile)");
        categoryChanges.push({
          type: "createCategory",
          category: cat,
          categoryNumber: i + 1
        });
      } else {
        log("d", "Category " + cat.title + " already exists in " + cat.sourceFile);
      }

      // Collect bind modifications and additions
      for (var k = 0; k < cat.binds.length; k++) {
        var bindItem = cat.binds[k];

        if (bindItem.status === "modified") {
          categoryChanges.push({
            type: "modify",
            bind: bindItem,
            category: cat.title
          });
        } else if (bindItem.status === "new") {
          addChanges.push({
            type: "add",
            bind: bindItem,
            category: cat.title
          });
        }
      }
    }

    // Combine in correct order: deletions -> category ops -> additions
    // IMPORTANT: Reverse addChanges because sed 'Nr' inserts after line N,
    // so multiple inserts at the same line end up in reverse order
    pendingChanges = deleteChanges.concat(categoryChanges).concat(addChanges.reverse());

    log("i", "Pending changes: " + pendingChanges.length);
    for (var pc = 0; pc < pendingChanges.length; pc++) {
      log("d", "  Change " + pc + ": " + pendingChanges[pc].type);
    }

    if (pendingChanges.length === 0) {
      log("w", "No changes to save");
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
    if (files.length === 0) {
      processNextChange();
      return;
    }

    // Single backup file per config (overwrites previous backup)
    var commands = files.map(function(f) {
      var backupPath = f + ".backup";
      return "cp '" + escapeShell(f) + "' '" + escapeShell(backupPath) + "'";
    });

    backupProcess.command = ["sh", "-c", commands.join(" && ")];
    backupProcess.running = true;
  }

  Process {
    id: backupProcess
    running: false

    onRunningChanged: {
      if (running) {
        log("i", "backupProcess STARTED");
      }
    }

    onExited: function(exitCode) {
      log("i", "backupProcess exited with code: " + exitCode);
      if (exitCode === 0) {
        log("i", "Backups created successfully");
        root.processNextChange();
      } else {
        log("e", "Backup FAILED");
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
      log("i", "=== All changes processed ===");
      saveComplete(true, "All changes saved successfully");
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
      case "createCategory":
        createCategoryHeader(change.category, change.categoryNumber);
        break;
      case "deleteCategoryHeader":
        deleteCategoryHeader(change.category);
        break;
      default:
        log("w", "Unknown change type: " + change.type);
        processNextChange();
    }
  }

  function createCategoryHeader(category, categoryNumber) {
    // Create a new category header in the config file
    var homeDir = Quickshell.env("HOME");
    var targetFile = pluginApi?.pluginSettings?.hyprlandConfigPath || (homeDir + "/.config/hypr/hyprland.conf");
    targetFile = targetFile.replace(/^~/, homeDir);

    // Format: # N. Category Name
    var headerLine = "# " + categoryNumber + ". " + category.title;
    log("i", "Creating category header: " + headerLine);

    // Escape single quotes for printf
    var safeLine = headerLine.replace(/'/g, "'\\''");

    // Add blank line before header and the header itself
    var cmd = "printf '\\n%s\\n' '" + safeLine + "' >> '" + escapeShell(targetFile) + "'";
    log("d", "Create category command: " + cmd);

    writeProcess.command = ["sh", "-c", cmd];
    writeProcess.running = true;
  }

  function deleteCategoryHeader(category) {
    if (!category.sourceFile) {
      log("w", "Cannot delete category header without source file");
      processNextChange();
      return;
    }

    var title = category.originalTitle || category.title;
    log("i", "Deleting category header: " + title + " from " + category.sourceFile);

    // Match pattern: # N. TITLE (where N is a number)
    // Actually delete the line (backups are created before saving)
    // Use \| as delimiter to avoid conflicts with / in paths
    var escapedTitle = escapeForSedPattern(title);
    var pattern = "^#[[:space:]]*[0-9]*\\.?[[:space:]]*" + escapedTitle + "[[:space:]]*$";

    var cmd = "sed -i -E '\\|" + pattern + "|d' '" + escapeShell(category.sourceFile) + "'";
    log("d", "Delete category header command: " + cmd);

    writeProcess.command = ["sh", "-c", cmd];
    writeProcess.running = true;
  }

  function modifyBind(bind) {
    if (!bind.sourceFile || !bind.rawLine) {
      log("w", "Cannot modify bind without source info");
      processNextChange();
      return;
    }

    var newLine = formatHyprlandBind(bind);
    log("d", "Modifying bind, new line: " + newLine);

    // Pattern (left side) needs full escaping including $
    var oldLine = escapeForSedPattern(bind.rawLine.trim());
    // Replacement (right side) should NOT escape $ (keeps $mod as literal)
    var newLineEsc = escapeForSedReplacement(newLine);

    log("d", "Old line escaped: " + oldLine);
    log("d", "New line escaped: " + newLineEsc);

    var cmd = "sed -i 's|^" + oldLine + "$|" + newLineEsc + "|' '" + escapeShell(bind.sourceFile) + "'";
    log("d", "Modify command: " + cmd);

    writeProcess.command = ["sh", "-c", cmd];
    writeProcess.running = true;
  }

  function addBind(bind, category) {
    // Determine target file - use main config if no source
    var homeDir = Quickshell.env("HOME");
    var targetFile = pluginApi?.pluginSettings?.hyprlandConfigPath || (homeDir + "/.config/hypr/hyprland.conf");
    targetFile = targetFile.replace(/^~/, homeDir);

    var newLine = formatHyprlandBind(bind);
    log("d", "Adding bind to category: " + category);
    log("d", "Target file: " + targetFile);
    log("d", "New line: " + newLine);

    // Try to find category header and insert after it, or append to end
    var categoryPattern = "# [0-9]*\\. " + category;
    var cmd = "grep -n '" + categoryPattern + "' '" + escapeShell(targetFile) + "' | tail -1 | cut -d: -f1";

    log("d", "Category search pattern: " + categoryPattern);
    log("d", "Search command: " + cmd);

    findCategoryProcess.targetFile = targetFile;
    findCategoryProcess.newLine = newLine;
    findCategoryProcess.command = ["sh", "-c", cmd];
    findCategoryProcess.running = true;
  }

  Process {
    id: findCategoryProcess
    property string targetFile: ""
    property string newLine: ""
    property string lineNumber: ""
    running: false

    stdout: SplitParser {
      onRead: data => findCategoryProcess.lineNumber = data.trim()
    }

    onExited: function(exitCode) {
      log("d", "Category search result: line=" + lineNumber + ", exitCode=" + exitCode);
      log("d", "newLine to write: " + newLine);

      // Escape single quotes for printf - this is the ONLY escaping needed
      // Inside single quotes: $mod stays as $mod, " stays as "
      var safeLine = newLine.replace(/'/g, "'\\''");
      log("d", "safeLine after quote escape: " + safeLine);

      var cmd;
      if (lineNumber && lineNumber !== "") {
        // Insert after category header using temp file + sed 'r' command
        // This avoids sed's a\ command which can interpret backslash sequences
        log("d", "Found category at line " + lineNumber + ", inserting after");
        var tempFile = "/tmp/hyprland_bind_" + Date.now();
        // Write to temp file, then use sed 'r' to read it after line N, then cleanup
        cmd = "printf '%s\\n' '" + safeLine + "' > '" + tempFile + "' && " +
              "sed -i '" + lineNumber + "r " + tempFile + "' '" + root.escapeShell(targetFile) + "' && " +
              "rm -f '" + tempFile + "'";
      } else {
        // Append to end of file using printf
        log("d", "Category not found, appending to end of file");
        cmd = "printf '%s\\n' '" + safeLine + "' >> '" + root.escapeShell(targetFile) + "'";
      }

      log("d", "Insert command: " + cmd);
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

    var rawLineTrimmed = bind.rawLine.trim();
    // Pattern needs full escaping for matching
    var patternLine = escapeForSedPattern(rawLineTrimmed);

    // Actually delete the line (backups are created before saving)
    // Use \| as delimiter to avoid conflicts with / in paths
    var cmd = "sed -i '\\|^" + patternLine + "$|d' '" + escapeShell(bind.sourceFile) + "'";
    log("d", "Delete command: " + cmd);

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

    // Match pattern: # N. OLD_TITLE (where N is a number)
    // Replace with: # N. NEW_TITLE
    // Use escapeForSedPattern for pattern, escapeForSedReplacement for replacement
    var oldPattern = "^(#[[:space:]]*[0-9]+\\.[[:space:]]*)" + escapeForSedPattern(oldTitle) + "[[:space:]]*$";
    var newReplacement = "\\1" + escapeForSedReplacement(newTitle);

    var cmd = "sed -i -E 's|" + oldPattern + "|" + newReplacement + "|' '" + escapeShell(sourceFile) + "'";

    log("d", "Rename command: " + cmd);

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

  function formatHyprlandBind(bind) {
    // Format: bind = MODS, KEY, ACTION #"DESCRIPTION"
    var modVar = pluginApi?.pluginSettings?.modKeyVariable || "$mod";

    var mods = [];
    if (bind.modifiers) {
      for (var i = 0; i < bind.modifiers.length; i++) {
        var mod = bind.modifiers[i];
        if (mod === "Super") mods.push(modVar);
        else if (mod === "Ctrl") mods.push("CTRL");
        else if (mod === "Shift") mods.push("SHIFT");
        else if (mod === "Alt") mods.push("ALT");
      }
    }

    var modsStr = mods.join(" ");
    var action = bind.action || "exec,";

    return "bind = " + modsStr + ", " + bind.key + ", " + action + " #\"" + bind.description + "\"";
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
    // We keep $mod as literal since 'm' is not a digit
    return str
      .replace(/\\/g, '\\\\')      // Backslash first
      .replace(/\|/g, '\\|')       // Pipe (our delimiter)
      .replace(/&/g, '\\&')        // Ampersand (inserts matched text)
      .replace(/\n/g, '')          // Remove newlines
      .replace(/\t/g, ' ')         // Convert tabs to spaces
      .replace(/\r/g, '');         // Remove carriage returns
  }

  // Keep old name for backward compatibility
  function escapeForSed(str) {
    return escapeForSedPattern(str);
  }

  function escapeShell(str) {
    // Comprehensive shell escaping for safe command execution
    // Escape all potentially dangerous shell characters
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
