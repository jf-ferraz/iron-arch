if status is-interactive
    # Commands to run in interactive sessions can go here
end

# Set cursor theme for niri compositor
set -gx XCURSOR_THEME "Bibata-Modern-Ice"
set -gx XCURSOR_SIZE "24"

# Add scripts directory to PATH
fish_add_path $HOME/.config/scripts

# Add Flutter to PATH
fish_add_path $HOME/development/flutter/bin

# Set Chrome executable for Flutter web development
set -gx CHROME_EXECUTABLE /usr/bin/google-chrome-stable
