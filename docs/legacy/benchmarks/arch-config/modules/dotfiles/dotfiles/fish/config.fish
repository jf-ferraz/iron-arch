if status is-interactive
    # Commands to run in interactive sessions can go here

    # Initialize zoxide
    zoxide init fish | source
end

# Alias for DCLI Sync
alias ds='dcli sync'

# Alias for opencode
alias op='opencode'

# Alias for Claude
alias cc='claude'

# Alias for felix
alias fx='felix'

# Alias for yazi
alias y='yazi'

# Alias for helix
alias hx='helix'

# Set cursor theme for niri compositor
set -gx XCURSOR_THEME Bibata-Modern-Ice
set -gx XCURSOR_SIZE 24

# Add scripts directory to PATH
fish_add_path $HOME/.config/scripts

# Add Flutter to PATH
fish_add_path $HOME/development/flutter/bin

# Set Chrome executable for Flutter web development
set -gx CHROME_EXECUTABLE /usr/bin/chromium

# Set default editor
set -gx EDITOR helix
