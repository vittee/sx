//! Ready-made snippets that `sx init` creates, all off until you turn them on in Profiles: the vim
//! ones and the shell ones (`colorterm`, `no-history`, `man-colors` and more) and the readline ones.
//!
//! `vim -u` skips the server's own vimrc, and the enabled `.vim` snippets are joined in name order
//! into one config, so these are small layers. A later name wins when two set the same option.
//! `vim-recommended` includes everything in `vim-basic`, so pick one of the two.

const BASIC: &str = r#"" sx vim-basic: the settings most people expect, no plugins
set nocompatible
syntax on
filetype plugin indent on
set encoding=utf-8
set backspace=indent,eol,start
set number
set ruler
set showcmd
set laststatus=2
set autoindent
set incsearch
set hlsearch
"#;

const RECOMMENDED_EXTRA: &str = r#"
" sx vim-recommended: comfort on top of the basics
set wildmenu
set wildmode=longest:full,full
set ignorecase
set smartcase
set smarttab
set scrolloff=5
set sidescrolloff=5
set display+=lastline
set nrformats-=octal
set ttimeoutlen=50
set history=1000
set showmatch
set listchars=tab:>-,trail:-,nbsp:+
set list
set splitright
set splitbelow
" Enter clears the search highlight
nnoremap <silent> <CR> :nohlsearch<CR><CR>
" j and k move by screen line in wrapped text
nnoremap j gj
nnoremap k gk
" keep the selection when indenting in visual mode
vnoremap < <gv
vnoremap > >gv
"#;

const INDENT_2: &str = r#"" sx vim-indent-2: two spaces
set expandtab
set shiftwidth=2
set softtabstop=2
set tabstop=2
"#;

const INDENT_4: &str = r#"" sx vim-indent-4: four spaces
set expandtab
set shiftwidth=4
set softtabstop=4
set tabstop=4
"#;

const INDENT_TABS: &str = r#"" sx vim-indent-tabs: real tabs, shown 8 wide (Makefiles, Go, kernel style)
set noexpandtab
set shiftwidth=8
set softtabstop=0
set tabstop=8
"#;

const MOUSE_CLIPBOARD: &str = r#"" sx vim-mouse: mouse and system clipboard when the server vim has them
set mouse=a
if has('clipboard')
  set clipboard=unnamed,unnamedplus
endif
"#;

const NO_TRACES: &str = r#"" sx vim-no-traces: vim leaves no files on the server
set noswapfile
set nobackup
set nowritebackup
set noundofile
set viminfo=
"#;

const KEYS: &str = r#"" sx vim-keys: a few common mappings
let mapleader = ' '
inoremap jk <Esc>
nnoremap <leader>w :w<CR>
nnoremap <leader>q :q<CR>
nnoremap <C-h> <C-w>h
nnoremap <C-j> <C-w>j
nnoremap <C-k> <C-w>k
nnoremap <C-l> <C-w>l
nnoremap <leader>n :set number!<CR>
nnoremap <leader>p :set paste!<CR>
"#;

const DARK: &str = r#"" sx vim-dark: colors for a dark terminal
set background=dark
set t_Co=256
silent! colorscheme desert
"#;

const COLORTERM: &str = "# sx colorterm: tell programs on the server that your terminal has 24-bit color
export COLORTERM=truecolor
";

const NO_HISTORY: &str = "# sx no-history: this shell does not write ~/.bash_history on the server
unset HISTFILE
";

const MAN_COLORS: &str = r#"# sx man-colors: colored man pages, and less keeps colors and ignores case when searching
export GROFF_NO_SGR=1
export LESS='-iR'
export LESS_TERMCAP_mb=$'\e[1;31m'
export LESS_TERMCAP_md=$'\e[1;36m'
export LESS_TERMCAP_me=$'\e[0m'
export LESS_TERMCAP_so=$'\e[1;33;44m'
export LESS_TERMCAP_se=$'\e[0m'
export LESS_TERMCAP_us=$'\e[1;32m'
export LESS_TERMCAP_ue=$'\e[0m'
"#;

const CMD_COLORS: &str = r#"# sx cmd-colors: color for ls, grep and diff when the output is a terminal (only what the server supports)
export CLICOLOR=1
[ -n "${LS_COLORS-}" ] || { command -v dircolors >/dev/null 2>&1 && eval "$(dircolors -b)"; }
ls --color=auto / >/dev/null 2>&1 && alias ls='ls --color=auto'
echo x | grep --color=auto -q x 2>/dev/null && alias grep='grep --color=auto'
diff --color=auto /dev/null /dev/null >/dev/null 2>&1 && alias diff='diff --color=auto'
"#;

const UTF8: &str = r#"# sx utf8: switch to a UTF-8 locale when the shell is not in one, so box drawing and accents show
case $(locale charmap 2>/dev/null) in
  UTF-8) ;;
  *)
    for __sx_l in C.UTF-8 C.utf8 en_US.UTF-8 en_US.utf8; do
      locale -a 2>/dev/null | grep -qix "$__sx_l" && { export LANG=$__sx_l LC_ALL=$__sx_l; break; }
    done
    unset __sx_l;;
esac
"#;

const DIR_KEYS: &str = r#"# sx dir-keys: Alt-Up goes to the parent directory, Alt-Left and Alt-Right go back and forward
# What you had typed goes to the kill ring, Ctrl-Y brings it back.
__sx_dk_back=() __sx_dk_fwd=() __sx_dk_last=$PWD __sx_dk_nav=
__sx_dk_track() {
  local s=$?
  if [ "$PWD" != "$__sx_dk_last" ]; then
    if [ -n "$__sx_dk_nav" ]; then
      __sx_dk_nav=
    else
      __sx_dk_back+=("$__sx_dk_last")
      __sx_dk_fwd=()
      [ ${#__sx_dk_back[@]} -gt 100 ] && __sx_dk_back=("${__sx_dk_back[@]:1}")
    fi
    __sx_dk_last=$PWD
  fi
  return $s
}
__sx_dk_up() { cd .. 2>/dev/null; }
__sx_dk_prev() {
  local n=${#__sx_dk_back[@]} d=${PWD}
  [ "$n" -gt 0 ] || return 0
  __sx_dk_nav=1
  if cd -- "${__sx_dk_back[n-1]}" 2>/dev/null; then __sx_dk_fwd+=("$d"); else __sx_dk_nav=; fi
  __sx_dk_back=("${__sx_dk_back[@]:0:n-1}")
}
__sx_dk_next() {
  local n=${#__sx_dk_fwd[@]} d=${PWD}
  [ "$n" -gt 0 ] || return 0
  __sx_dk_nav=1
  if cd -- "${__sx_dk_fwd[n-1]}" 2>/dev/null; then __sx_dk_back+=("$d"); else __sx_dk_nav=; fi
  __sx_dk_fwd=("${__sx_dk_fwd[@]:0:n-1}")
}
case $PROMPT_COMMAND in *__sx_dk_track*) ;; *) PROMPT_COMMAND="__sx_dk_track${PROMPT_COMMAND:+;$PROMPT_COMMAND}";; esac
bind -x '"\C-x\C-u": __sx_dk_up'
bind -x '"\C-x\C-b": __sx_dk_prev'
bind -x '"\C-x\C-n": __sx_dk_next'
bind '"\e[1;3A": "\C-a\C-k\C-x\C-u\n"'
bind '"\e\e[A": "\C-a\C-k\C-x\C-u\n"'
bind '"\e[1;3D": "\C-a\C-k\C-x\C-b\n"'
bind '"\e\e[D": "\C-a\C-k\C-x\C-b\n"'
bind '"\e[1;3C": "\C-a\C-k\C-x\C-n\n"'
bind '"\e\e[C": "\C-a\C-k\C-x\C-n\n"'
"#;

/// Shell snippets created on the first run, off until you turn them on. ssh does not send
/// `COLORTERM`, so vim and `less` on a server fall back to 256 colors. Turn `colorterm` on only when
/// your terminal supports truecolor (Windows Terminal, iTerm2, kitty and WezTerm do). `no-history`
/// is for servers that keep `~/.bash_history` empty on purpose: `sx` starts bash without the login
/// files that usually say so. The rest are small comforts for the command line and the terminal. They touch only this shell.
pub(super) fn shell_presets() -> Vec<(&'static str, String)> {
    vec![
        ("cmd-colors", CMD_COLORS.to_string()),
        ("colorterm", COLORTERM.to_string()),
        ("dir-keys", DIR_KEYS.to_string()),
        ("man-colors", MAN_COLORS.to_string()),
        ("no-history", NO_HISTORY.to_string()),
        ("utf8", UTF8.to_string()),
    ]
}

const COMPLETION_COLORS: &str = "# sx completion-colors: Tab completion lists use your LS_COLORS, and the part you typed is colored
# The server needs LS_COLORS for the first one: turn on cmd-colors if it has none.
# Readline older than 6.3 (7.0 for the prefix) does not know these, so they are skipped there.
$if version >= 6.3
set colored-stats on
$endif
$if version >= 7.0
set colored-completion-prefix on
$endif
";

const HISTORY_SEARCH: &str = r#"# sx history-search: Up and Down search the history for what you typed so far
"\e[A": history-search-backward
"\e[B": history-search-forward
"\eOA": history-search-backward
"\eOB": history-search-forward
"#;

const KEYS_FIX: &str = r#"# sx keys-fix: Home, End, Delete and Ctrl-Left/Right work the same on every server
"\e[1~": beginning-of-line
"\e[4~": end-of-line
"\e[7~": beginning-of-line
"\e[8~": end-of-line
"\e[H": beginning-of-line
"\e[F": end-of-line
"\eOH": beginning-of-line
"\eOF": end-of-line
"\e[3~": delete-char
"\e[1;5C": forward-word
"\e[1;5D": backward-word
"\e[5C": forward-word
"\e[5D": backward-word
"#;

const MENU_COMPLETE: &str = r#"# sx menu-complete: Tab cycles through the matches, Shift-Tab goes back
# The first Tab still completes the part they share, and the next ones cycle.
"\t": menu-complete
"\e[Z": menu-complete-backward
$if version >= 6.3
set menu-complete-display-prefix on
$endif
"#;

const NO_BELL: &str = "# sx no-bell: no beep or flash when a completion fails or the cursor hits an edge
set bell-style none
";

const SUDO_ESC: &str = r#"# sx sudo-esc: press Esc twice to put sudo in front of the command line
# Only with emacs keys: in vi mode Esc leaves insert mode and a second one must not be taken.
# Esc Esc starts longer sequences too (Alt with an arrow key), so readline waits for more keys.
# The default wait is half a second, and 100 ms is enough for a key press that arrives in one piece.
$if mode=emacs
"\e\e": "\C-asudo \C-e"
set keyseq-timeout 100
$endif
"#;

/// Readline key bindings created on the first run. They are `.inputrc` snippets, so `sx` applies them
/// with `bind`, one line at a time, when the shell starts.
pub(super) fn inputrc_presets() -> Vec<(&'static str, String)> {
    vec![
        ("completion-colors.inputrc", COMPLETION_COLORS.to_string()),
        ("history-search.inputrc", HISTORY_SEARCH.to_string()),
        ("keys-fix.inputrc", KEYS_FIX.to_string()),
        ("menu-complete.inputrc", MENU_COMPLETE.to_string()),
        ("no-bell.inputrc", NO_BELL.to_string()),
        ("sudo-esc.inputrc", SUDO_ESC.to_string()),
    ]
}

/// Names and texts of the vim snippets created on the first run. `vim-recommended` is built from
/// `vim-basic` plus its own extras.
pub(super) fn vim_presets() -> Vec<(&'static str, String)> {
    vec![
        ("vim-basic.vim", BASIC.to_string()),
        ("vim-dark.vim", DARK.to_string()),
        ("vim-indent-2.vim", INDENT_2.to_string()),
        ("vim-indent-4.vim", INDENT_4.to_string()),
        ("vim-indent-tabs.vim", INDENT_TABS.to_string()),
        ("vim-keys.vim", KEYS.to_string()),
        ("vim-mouse.vim", MOUSE_CLIPBOARD.to_string()),
        ("vim-no-traces.vim", NO_TRACES.to_string()),
        ("vim-recommended.vim", format!("{BASIC}{RECOMMENDED_EXTRA}")),
    ]
}
