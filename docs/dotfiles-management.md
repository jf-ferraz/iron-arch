# Managing Dotfiles on Arch with Iron

> How daily dotfile management works once you move off NixOS/home-manager onto
> Arch + Iron. Answers the core question: **"Do I edit Nix, or do I edit the
> `.toml`/`.conf` directly?"** — and shows the folder structure that makes it work.

---

## 1. The mental model: where does "truth" live?

On NixOS, the source of truth is the Nix expression. `~/.config/kitty/kitty.conf`
is a symlink into the **read-only** `/nix/store`. You cannot edit it in place — to
change a keybind you edit `home/programs/kitty/*.nix` and run `nixos-rebuild`. The
generated file is an *output*, never something you touch.

Iron inverts this. With Iron, the source of truth is a **plain file in a writable
git repo** (`~/.config/iron`). When a module declares a dotfile with `link = true`,
Iron creates:

```
~/.config/kitty   →  ~/.config/iron/modules/kitty/config/kitty   (symlink)
```

That symlink points into **your repo**, which is writable. So when you open
`~/.config/kitty/kitty.conf` in your editor and save, **you are editing the repo
file** — same inode. The change is already version-controlled the moment you save;
all that's left is `git commit`. No rebuild, no re-apply, no "compile" step.

This is the single most important difference for daily life:

| | NixOS / home-manager | Iron (symlink module) |
|---|---|---|
| `~/.config/X` points to | read-only `/nix/store` | your writable repo |
| To change a setting | edit `.nix` → `nixos-rebuild` | edit the file → `git commit` |
| Test a tweak | rebuild (seconds–minutes) | save & relaunch the app (instant) |
| Source of truth | Nix expression (generator) | the actual config file |

**So the answer to "Nix or `.conf`?" is: you edit the `.conf` directly, and you
stop touching Nix entirely.** `nix-config` becomes either retired or — during
migration only — a one-time *compiler* (see §6).

There is exactly **one exception**, and it's the whole reason the rest of this
document exists: **themed/templated files.**

---

## 2. The one nuance: symlinked files vs templated files

Iron deploys a dotfile in one of two modes. The mode decides *where you edit it*.

### Mode A — Symlink (`link = true`) — "edit live"
The live file **is** the repo file. Edit either path; they're the same bytes.
Activation cost: **zero** (just commit). This is how you want ~90 % of your
configs: anything you tweak by hand and that doesn't need shared theme colors.

### Mode B — Template (rendered) — "edit the template, then apply"
When a config needs values from a shared palette (base16 colors, fonts) — the job
Stylix did on NixOS — the repo holds a **`.tmpl`** file with placeholders, and Iron
**renders a real copy** into `~/.config`. The live file is now a *generated output*
again (like Nix). If you edit it live, the next `iron apply` overwrites it. So for
themed files you edit the **`.tmpl` in the repo** and run `iron apply` to re-render.

```
modules/kitty/config/kitty/kitty.conf.tmpl   ← you edit THIS (has {{base00}} etc.)
        │  iron apply  (renders palette in)
        ▼
~/.config/kitty/kitty.conf                    ← generated copy, do NOT edit live
```

> **Stylix parity note.** Mode B is the Iron-native replacement for Stylix. It
> depends on the theme-palette + template-engine feature (see
> `dotfiles-management` → *implications* and the roadmap). Until that lands, themed
> files are managed as frozen static symlinks instead (you lose live re-theming but
> keep everything else).

### The rule of thumb
> **Plain config you hand-tune → Mode A (symlink, edit live).**
> **Config whose colors/fonts come from the shared palette → Mode B (template, edit `.tmpl` + `iron apply`).**

This split is *exactly* how your `nix-config` already divides the world: files that
reference `config.lib.stylix.colors` (walker, niri, gnome, fastfetch, lazygit, fzf)
become Mode B; everything else (fish functions, neovim, helix, git, tmux, yazi)
becomes Mode A.

---

## 3. The folder structure

One git repo, conventionally cloned to `~/.config/iron`, is the entire system.

```
~/.config/iron/                      # ← THE repo. git is your history & sync.
│
├── hosts/                           # machine-specific facts (hardware, which bundle/profile)
│   ├── desktop.toml                 #   the powerful Arch desktop
│   └── notebook.toml                #   (optional) keep the ThinkPad here too
│
├── bundles/                         # ── SYSTEM / DESKTOP-ENV layer ──
│   ├── niri/                        #   == nix-config modules/desktop/niri
│   │   ├── bundle.toml              #     packages + systemd services + conflicts
│   │   ├── config/niri/config.kdl[.tmpl]   #   Mode B if themed
│   │   └── scripts/setup-niri.sh    #     post-install hook
│   ├── hyprland/ …
│   └── gnome/ …
│
├── profiles/                        # ── COMPOSITION layer (the "home" set) ──
│   ├── developer/profile.toml       #   modules = [...]; extends = "minimal"; theme = "..."
│   └── minimal/profile.toml         #   == nix-config home/desktop/* selection
│
├── modules/                         # ── PER-APP DOTFILES (where daily editing happens) ──
│   ├── kitty/
│   │   ├── module.toml              #   packages=["kitty"]; [[dotfiles]] source/target/link
│   │   └── config/kitty/
│   │       └── kitty.conf[.tmpl]    #   Mode A (live) or Mode B (themed)
│   ├── fish/
│   │   ├── module.toml
│   │   └── config/fish/
│   │       ├── config.fish          #   Mode A — edit live, commit
│   │       └── functions/*.fish
│   ├── neovim/  config/nvim/…        #   Mode A (see §6 note on nvf)
│   ├── helix/   config/helix/…       #   Mode A
│   ├── yazi/    config/yazi/…        #   Mode A
│   ├── git/     config/git/…         #   Mode A
│   ├── fastfetch/ config/fastfetch/config.jsonc.tmpl   #   Mode B (uses palette)
│   ├── walker/  config/walker/…tmpl  #   Mode B (heavily themed)
│   └── …
│
├── themes/                          # ── THE STYLIX-EQUIVALENT ──
│   ├── catppuccin-mocha.toml        #   base16 palette + font set (the single source of color)
│   └── gruvbox.toml                 #   switch the whole system by changing profile.theme
│
├── secrets/
│   └── secrets.yaml                 #   age/sops or git-crypt encrypted
│
└── state.json / .state.lock         # ← Iron-managed. NEVER hand-edit. Not your truth; it's a cache of "what I applied".
```

### What each file actually contains

A **module** = "one app": what to install + what files to deploy + optional hook.

```toml
# modules/kitty/module.toml
id = "kitty"
name = "Kitty Terminal"
kind = "AppConfig"
packages = ["kitty"]              # pacman installs these on `iron apply`

[[dotfiles]]
source = "config/kitty"           # path inside this module dir
target = "~/.config/kitty"        # where it lands in $HOME
link   = true                     # true = Mode A (symlink). Drop/false + .tmpl = Mode B.
```

A **theme** = the palette every Mode-B file pulls from (proposed format):

```toml
# themes/catppuccin-mocha.toml
[palette]                          # base16
base00 = "1e1e2e"   # bg
base05 = "cdd6f4"   # fg
base0D = "89b4fa"   # accent/blue
# … base01..base0F …

[fonts]
mono  = "JetBrainsMono Nerd Font"
sans  = "Inter"
size  = 11
```

A **profile** = which modules + which theme, with inheritance:

```toml
# profiles/developer/profile.toml
id = "developer"
extends = "minimal"                # inherit minimal's modules
modules = ["kitty", "fish", "neovim", "helix", "walker", "fastfetch", "git"]
theme   = "catppuccin-mocha"       # ← selects themes/catppuccin-mocha.toml for all Mode-B files
shell   = "fish"
```

A **host** = hardware + which bundle/profile this machine runs:

```toml
# hosts/desktop.toml
id = "desktop"
bundle  = "niri"                   # or hyprland / gnome
profile = "developer"
[variables]                        # per-machine template vars (monitor, etc.)
primary_monitor = "DP-1"
```

---

## 4. A day in the life (concrete workflows)

**① Tweak a kitty keybind (Mode A file).**
```bash
$EDITOR ~/.config/kitty/kitty.conf     # it's a symlink into the repo
# …add the binding, save, kitty picks it up live…
cd ~/.config/iron && git commit -am "kitty: add tab nav binding"
```
No `iron apply`. The symlink means the repo already has the change.

**② Add a fish alias / function (Mode A).** Same as above — edit
`~/.config/fish/...`, it's live, commit. Nothing to rebuild.

**③ Change the system color scheme (Mode B, the Stylix moment).**
```bash
$EDITOR ~/.config/iron/profiles/developer/profile.toml   # theme = "gruvbox"
iron apply                                                # re-renders EVERY .tmpl with the new palette
```
One edit, every themed app (kitty colors, niri, walker, waybar, fastfetch, fzf,
lazygit) re-rendered in one shot. This is the capability you'd be sad to lose, and
it's why Mode B exists.

**④ Add a brand-new app (e.g. `zellij`).**
```bash
mkdir -p ~/.config/iron/modules/zellij/config/zellij
$EDITOR ~/.config/iron/modules/zellij/module.toml        # id, packages=["zellij"], [[dotfiles]]
$EDITOR ~/.config/iron/modules/zellij/config/zellij/config.kdl
# add "zellij" to profiles/developer/profile.toml modules=[...]
iron apply                                               # pacman installs it + symlinks the config
git commit -am "add zellij module"
```

**⑤ Change what a module installs / enable a service.** Edit the `module.toml`
(or bundle `services = [...]`) → `iron apply`. Structural changes always need apply;
file-content changes to Mode-A files never do.

**⑥ Switch desktop environment.** `iron bundle switch hyprland` — swaps the system
layer; your profile/modules (the dotfiles) ride along unchanged.

### The decision table (memorize this)

| You are changing… | Edit here | Activate with |
|---|---|---|
| A plain app's settings (keybind, alias, option) | the live `~/.config/<app>/…` file (Mode A symlink) | nothing — `git commit` |
| A themed app's **colors** (all at once) | `profiles/*/profile.toml` `theme =` (or `themes/<name>.toml`) | `iron apply` |
| A themed app's **non-color content** | the `…​.tmpl` in the repo (Mode B) | `iron apply` |
| Which **packages** an app needs | `modules/<id>/module.toml` | `iron apply` |
| **Add/remove** a whole app | `modules/<id>/` + the profile's `modules=[]` | `iron apply` |
| A **systemd service** on/off | bundle/module `services=[]` | `iron apply` |
| **Desktop environment** | — | `iron bundle switch <id>` |
| **Per-machine** value (monitor, hostname) | `hosts/<id>.toml` `[variables]` | `iron apply` |

**You never edit `nix-config` in any of these rows.** That's the whole point.

---

## 5. Implications & applications

**Live-edit ergonomics beat home-manager (for Mode-A files).** No rebuild to test a
keybind. The edit↔test loop is as fast as raw dotfiles, because it *is* raw
dotfiles — just symlinked and git-tracked. This is the daily-driver win.

**Theming stays powerful (for Mode-B files).** One palette → many files reproduces
the Stylix experience. The cost: themed files give up live-editing — you trade
immediacy for the shared-palette superpower. You choose, per file, which trade you
want. That choice is the `link = true` vs `.tmpl` decision.

**Multi-machine.** Same repo, `git push`/`pull`. Machine differences live in
`hosts/<id>.toml` (`bundle`, `profile`, `extra_modules`, `[variables]`). The
desktop runs `niri + developer`; the ThinkPad could run `niri + developer` with a
different monitor variable, or a leaner profile. `iron apply` on each box reconciles
to that host's declaration.

**Git is your generation history — and it's better here than on Nix in one way.**
Every dotfile change is a normal commit with a normal diff. `git log -p
modules/kitty/` shows the real history of your kitty config. NixOS generations
roll back the *whole system* but don't give you readable per-file dotfile diffs;
Iron does, because the dotfiles are first-class files in the repo.

**Rollback is two-layered.** (a) `git revert`/`checkout` for config content, then
`iron apply`. (b) Timeshift/Snapper snapshots (Iron takes one pre-apply) for the
system/package layer. Note the honest gap: Iron's snapshot *creation* is solid, but
*automated* package/dotfile rollback execution is still partial — practically,
rollback today = `git` for files + snapshot restore for packages.

**The reproducibility caveat (be honest with yourself).** Iron is **not** hermetic
the way Nix is. Today it's *additive* for packages: it installs what you declare and
prunes things it has tracked, but it does **not** automatically remove packages you
installed by hand outside Iron. So `iron apply` on a fresh machine reproduces your
declared set, but a long-lived machine can accumulate undeclared drift. (Closing
this — `actual − desired` package pruning — is a worthwhile Iron improvement; the
action plumbing already exists.) If bit-for-bit reproducibility matters more than
daily ergonomics, that's the one axis where you're giving something up vs NixOS.

**Secrets.** Your Nix setup uses sops + age. Iron ships git-crypt today. Either
re-encrypt to git-crypt, or add an age/sops backend to Iron so `secrets/secrets.yaml`
works unchanged. Until then, keep secrets out of the symlinked tree.

---

## 6. Migrating your existing `nix-config`

You don't hand-rewrite everything. Home-manager can **render** its generated files,
and you harvest them into modules:

```bash
home-manager build --flake ~/nix-config#fer
# result/home-files/.config/{kitty,fish,helix,fastfetch,nvim,…} = fully-rendered configs,
# WITH Stylix colors already baked in.
```

Then, per app, drop the rendered output into `modules/<app>/config/<app>/` and write
a one-line `module.toml`. Mark it Mode A (symlink) unless it's a themed file you want
to keep re-theming, in which case re-introduce `{{base00}}`-style placeholders and
make it a `.tmpl` (Mode B).

**Per-app landing table:**

| nix-config source | Iron module | Mode |
|---|---|---|
| `home/programs/kitty` | `modules/kitty` | B (colors) — or A if you freeze the theme |
| `home/programs/fish` | `modules/fish` | A |
| `home/programs/helix` | `modules/helix` | A |
| `home/programs/yazi` | `modules/yazi` | A |
| `home/programs/neovim` (**nvf**) | `modules/neovim` | A — **frozen** (see note) |
| `home/programs/zed` (nix-generated) | `modules/zed` | A — **frozen** |
| `home/programs/fastfetch` | `modules/fastfetch` | B (colors) |
| `home/desktop/walker.nix` | `modules/walker` | B (heavily themed) |
| `home/desktop/niri` | `bundles/niri` config | B (colors) |
| Stylix palette | `themes/<name>.toml` | — (the new source of color) |
| `modules/desktop/*` (system) | `bundles/*` + pacman | — |

> **Frozen-config note (nvf, zed-from-nix).** Configs that Nix *generates from a DSL*
> (nvf builds a big `init.lua`; your zed settings come from Nix) harvest as a single
> rendered file. That works, but it's a **frozen snapshot** — you've left the nvf DSL
> behind, so future edits are hand-edits to the generated config (Mode A live-edit,
> which is arguably nicer day-to-day, but you lose nvf's abstraction). Decide whether
> these apps are worth re-authoring natively or are fine frozen.

**Suggested order:** keep the NixOS box as a reference *compiler* during the
transition. Migrate the cheap Mode-A apps first (fish, helix, git, yazi, neovim),
confirm the daily loop feels right, then tackle Mode-B theming once Iron's palette +
template engine is in place. Retire `nix-config` from the Arch box only when no
module still needs it to compile.

---

## TL;DR

- **You edit the `.conf`/`.toml`, not Nix.** `nix-config` is retired (or a one-time
  compiler during migration).
- **Plain configs (Mode A):** `~/.config/X` is a symlink into `~/.config/iron`, so
  you edit it live and just `git commit`. No apply, no rebuild.
- **Themed configs (Mode B):** edit the `.tmpl` (or the `theme`), run `iron apply` —
  one palette re-renders every themed app. This is your Stylix replacement.
- **Structure:** one repo at `~/.config/iron` → `hosts/ bundles/ profiles/ modules/
  themes/ secrets/`. Modules hold per-app dotfiles; that's where you live.
- **Trade-off vs NixOS:** faster daily edits, readable per-file git history, no
  nixGL pain — at the cost of hermetic reproducibility (package drift) and, for
  frozen nix-generated configs, the original DSL abstraction.
