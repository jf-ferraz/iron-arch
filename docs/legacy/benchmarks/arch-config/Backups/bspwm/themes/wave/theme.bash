# ------------------------------------------------------------------------------
# Copyright (C) 2020-2025 Aditya Shakya <adi1090x@gmail.com>
#
# Wave Theme
# ------------------------------------------------------------------------------

# Source pywal colors if available, otherwise use defaults
if [[ -f "$HOME/.cache/wal/colors.sh" ]]; then
    source "$HOME/.cache/wal/colors.sh"

    # Use pywal generated colors
    background="$background"
    foreground="$foreground"
    color0="$color0"
    color1="$color1"
    color2="$color2"
    color3="$color3"
    color4="$color4"
    color5="$color5"
    color6="$color6"
    color7="$color7"
    color8="$color8"
    color9="$color9"
    color10="$color10"
    color11="$color11"
    color12="$color12"
    color13="$color13"
    color14="$color14"
    color15="$color15"

    # Set accent and elements based on pywal colors
    accent="$color5"
    element_bg="$color0"
    element_fg="$foreground"
else
    # Default fallback colors
    background='#323f4e'
    foreground='#f8f8f2'
    color0='#3d4c5f'
    color1='#f48fb1'
    color2='#a1efd3'
    color3='#f1fa8c'
    color4='#92b6f4'
    color5='#bd99ff'
    color6='#87dfeb'
    color7='#f8f8f2'
    color8='#56687e'
    color9='#ee4f84'
    color10='#53e2ae'
    color11='#f1ff52'
    color12='#6498ef'
    color13='#985eff'
    color14='#24d1e7'
    color15='#e5e5e5'

    accent='#BD99FF'
    element_bg='#3D4C5F'
    element_fg='#F8F8F8'
fi

light_value='0.12'
dark_value='0.30'

# Wallpaper
wdir="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
wallpaper="/home/don/.config/arch-config/wallpapers/venom.jpg"

# Polybar
polybar_font='Iosevka:size=10;3'

# Rofi
rofi_font='Iosevka 10'
rofi_icon='Luv-Folders-Dark'

# Terminal
terminal_font_name='JetBrainsMono Nerd Font'
terminal_font_size='10'

# Geany
geany_colors='wave.conf'
geany_font='JetBrainsMono Nerd Font 10'

# Appearance
gtk_font='Noto Sans 9'
gtk_theme='Catppuccin-Mocha'
icon_theme='Tela-circle-purple-dark'
cursor_theme='Vimix'

# Dunst
dunst_width='300'
dunst_height='80'
dunst_offset='20x58'
dunst_origin='bottom-right'
dunst_font='Iosevka Custom 9'
dunst_border='0'
dunst_separator='2'

# Picom
picom_backend='glx'
picom_corner='0'
picom_shadow_r='20'
picom_shadow_o='0.60'
picom_shadow_x='-20'
picom_shadow_y='-20'
picom_blur_method='none'
picom_blur_strength='0'

# Bspwm
bspwm_fbc="$accent"
bspwm_nbc="$background"
bspwm_abc="$color5"
bspwm_pfc="$color2"
bspwm_border='2'
bspwm_gap='10'
bspwm_sratio='0.50'
