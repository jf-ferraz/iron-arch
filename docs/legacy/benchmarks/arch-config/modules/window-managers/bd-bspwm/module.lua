local packages = {
    -- BSPWM window manager core
    "bspwm",
    "sxhkd",
    
    -- Terminal emulators
    "alacritty",
    "kitty",
    
    -- Theming and appearance
    "dunst",
    "picom",
    "rofi",
    "feh",
    "papirus-icon-theme",
    "redshift",
    
    -- Window management utilities
    "xclip",
    "xdo",
    "xdotool",
    "xsettingsd",
    "xorg-xdpyinfo",
    "xorg-xkill",
    "xorg-xprop",
    "xorg-xrandr",
    "xorg-xsetroot",
    "xorg-xwininfo",
    "xorg-xrdb",
    
    -- System utilities
    "bat",
    "brightnessctl",
    "clipcat",
    "eza",
    "fzf",
    "jq",
    "libwebp",
    "lxsession",
    "maim",
    "npm",
    "pamixer",
    "pacman-contrib",
    "playerctl",
    "python-gobject",
    "rustup",
    "yazi",
    
    -- File managers and related
    "dolphin",
    "tumbler",
    "gvfs-mtp",
    "simple-mtpfs",
    
    -- Media and music
    "geany",
    "imagemagick",
    "jgmenu",
    "mpc",
    "mpd",
    "mpv",
    "ncmpcpp",
    
    -- Fonts
    "ttf-inconsolata",
    "ttf-jetbrains-mono",
    "ttf-jetbrains-mono-nerd",
    "ttf-terminus-nerd",
    "ttf-ubuntu-mono-nerd",
    "webp-pixbuf-loader",
    
    -- Build dependencies
    "go",
    "base-devel",
    "git",
    
    -- Bars and widgets
    "polybar",
    
    -- Lock screen
    "betterlockscreen",
    
    -- Color picker
    "xcolor",
    
    -- Idle detection
    "xprintidle",
}

-- Packages from gh0stzk repository (requires gh0stzk-dotfiles repo)
local gh0stzk_packages = {
    "st-gh0stzk",
    "gh0stzk-gtk-themes",
    "gh0stzk-cursor-qogirr",
    "gh0stzk-icons-beautyline",
    "gh0stzk-icons-candy",
    "gh0stzk-icons-catppuccin-mocha",
    "gh0stzk-icons-dracula",
    "gh0stzk-icons-glassy",
    "gh0stzk-icons-gruvbox-plus-dark",
    "gh0stzk-icons-hack",
    "gh0stzk-icons-luv",
    "gh0stzk-icons-sweet-rainbow",
    "gh0stzk-icons-tokyo-night",
    "gh0stzk-icons-vimix-white",
    "gh0stzk-icons-zafiro",
    "gh0stzk-icons-zafiro-purple",
}

-- Packages from Chaotic AUR (requires chaotic-aur repo)
local chaotic_packages = {
    "eww-git",
}

-- AUR packages that need to be built
local aur_packages = {
    "xwinwrap-0.9-bin",
    "i3lock-color",
    "fzf-tab-git",
}

return {
    description = "BSPWM window manager with gh0stzk dotfiles and 18 themes",
    conflicts = {},
    dotfiles_sync = true,
    packages = packages,
    
    -- Additional metadata for documentation
    _extra = {
        gh0stzk_repo_packages = gh0stzk_packages,
        chaotic_repo_packages = chaotic_packages,
        aur_packages = aur_packages,
        required_repos = {
            "gh0stzk-dotfiles",
            "chaotic-aur",
        },
        preinstall_script = "./preinstall.sh",
    }
}
