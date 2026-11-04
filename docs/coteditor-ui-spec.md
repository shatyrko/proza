# CotEditor UI/UX Specification (for a GTK4 / libadwaita clone)

Reference: CotEditor `main` branch, commit `258fcf3a6ac09aff85e1c398b7787abad0e1c863` (2026-11-03), shallow clone. CotEditor is Apache-2.0. This document describes **behaviour and UI only**; it contains no source code. All English labels were taken from the storyboard and the `.xcstrings` localization catalogs (source-language values), so they are exact unless marked otherwise.

Markers used throughout:

- **[macOS-only]**: has no sensible Linux equivalent, or depends on a macOS system service. A suggested substitute is usually given.
- ⌘ ⌥ ⌃ ⇧: macOS modifier notation (Command, Option, Control, Shift). See "Main Menu → Conventions" for the suggested Linux mapping.
- "pt" = macOS points (≈ GTK logical pixels).
- SF Symbol names in brackets (e.g. `[gear]`) identify the macOS icon; pick the nearest Adwaita symbolic icon.

## Contents

1. Application Overview
2. Main Menu (every menu, item, separator, default shortcut)
3. Document Window Layout
4. Toolbar
5. Navigation Bar (Outline Navigator)
6. Editor View
7. Split Editor
8. Status Bar
9. Inspector (Document, Outline, Warnings)
10. File Browser Sidebar
11. Document Lifecycle (open, encodings, save, autosave, external changes, close, restore, tabs, rmate)
12. Open/Save/Print Accessories
13. Text Commands: Common Semantics
14. Find & Replace Panel
15. Multiple Replace
16. Find in Folder
17. Go To Line
18. Character Inspector
19. Unicode Input and Other Small Sheets (Tab Width, Surround, Sort by Pattern, Advanced Character Count, Color Code)
20. Snippets Behaviour
21. Scripts Menu
22. Console
23. Quick Actions (Command Bar)
24. Settings Window (General, Appearance, Window, Edit, Mode, Format, Snippets, Shortcuts, Integration)
25. macOS-only Features and Linux Substitutes
26. Clone Checklist: UX Details That Must Be Right

---


## Application Overview

### What CotEditor is

A document-based plain-text editor. Its focus is prose and code: syntax highlighting, an outline, encoding and line-ending handling, multi-cursor editing, regex find/replace, and user scripts. There is no project system and no build/run feature. Folders can be opened as "folder windows" that add a file-browser sidebar.

### Window kinds

| Window | Purpose | Notes |
|---|---|---|
| Document window (file) | One text document; can be tabbed with other document windows | Toolbar, editor(s) with navigation bars, status bar, optional inspector on the right |
| Document window (folder) | One folder; shows one file at a time from its tree | Same as above plus the left sidebar (File Browser / Folder Find); Back/Forward history; never tabbed |
| Find & Replace panel | Floating utility panel shared by all documents | Find All results appear inside it |
| Multiple Replace window | Edit and apply saved batches of replace rules | |
| Settings window | 9 toolbar panes | |
| Console | Floating panel with script output and errors | |
| Quick Actions | Spotlight-like command palette (⌘K) | |
| Advanced Character Count | Small floating counter panel | |
| Color Code panel | Colour picker with a code accessory | |
| About / What's New / Donation | Small informational panels | |

Popovers and sheets attached to a document window include Go To Line, Character Inspector, Unicode input, Custom Tab Width, Custom Surround, Sort by Pattern, encoding-change alerts, and the external-change alert.

### Suggested GTK4/libadwaita mapping

| CotEditor (AppKit) | GTK4/libadwaita suggestion |
|---|---|
| Window with unified toolbar (title + toolbar items) | `AdwApplicationWindow` + `AdwToolbarView` with an `AdwHeaderBar`. Toolbar items become header-bar buttons/menus. "Customize Toolbar" can be a simple checkbox dialog, or omitted. |
| Native window tabs | `AdwTabView` + `AdwTabBar` (one window, many documents). Keep "New Window" vs "New Tab". |
| Three-column split view (sidebar · content · inspector) | Nested `AdwOverlaySplitView`s, or a `GtkPaned` pair. The sidebar is on the start side (folder windows only), the inspector on the end side. Collapsed state and widths are remembered per window kind. |
| Editor | `GtkSourceView 5` (line numbers, invisibles via space drawer, right-margin page guide, current-line highlight, bracket matching, snippets, completion). Add custom gutter renderers where needed. |
| Navigation bar above each editor | A thin `GtkBox` (24 px) above each editor pane. |
| Status bar | A thin bottom bar in the content column (a bottom bar of `AdwToolbarView`). |
| Popovers / sheets | `GtkPopover`, `AdwDialog` / `AdwAlertDialog`. |
| Floating panels (Find, Console, Multiple Replace) | Separate non-modal `AdwWindow`s marked transient for the app; an in-window search bar is an acceptable UX deviation. |
| Settings window | `AdwPreferencesDialog`/`AdwPreferencesWindow` with pages; or an `AdwViewSwitcher` header bar to match the macOS toolbar-tab look. |
| Menu bar | `GtkPopoverMenuBar` (classic menubar, closest to CotEditor), or a primary menu plus header-bar menus. The menubar route keeps the full command set discoverable. |

### State scope: what is global and what is per window

This matters for UX consistency.

- **Global (saved as user defaults, applied to all windows live):** status bar visible, navigation bar visible, split orientation default, all Settings values, recent syntaxes, inspector selected pane index, Find panel strings and options.
- **Per window or document (initialised from Settings when a document opens, then changed from the Format/View menus or toolbar):**
  - Display toggles: show invisibles, line numbers, page guide, indent guides, wrap lines.
  - Indentation: tab width, soft tabs.
  - Text and layout: theme, font type (standard vs monospaced), antialias, ligatures, writing direction and orientation, zoom.
  - Document state: syntax, encoding, line ending, read-only flag.
  - Window chrome: inspector shown, Keep on Top.

  Changing the matching setting in Settings resets the per-window value in all open windows (see Editor View).
- Window restoration saves the per-window state.

### User data locations and file formats

macOS stores everything under `~/Library/Application Support/CotEditor/`, in these sub-folders. The Linux column gives a suggested XDG equivalent.

| Data | macOS folder | File type | Linux (suggested) |
|---|---|---|---|
| Themes | `Themes/` | `.cottheme` (JSON; colours as hex `#RRGGBB[AA]`, plus `usesSystemSetting` flags and metadata: author, description, license, URL) | `$XDG_DATA_HOME/coteditor-clone/themes/` |
| Syntaxes | `Syntaxes/` | `.cotsyntax` (a folder package containing `Info.json`, `Edit.json`, `Completion.json`, and highlight definitions for user syntaxes) | `…/syntaxes/` |
| Multiple Replace definitions | `Replacements/` | `.cotrpl` | `…/replacements/` |
| Customized menu key bindings | `KeyBindings/` | plist/JSON | `$XDG_CONFIG_HOME/…/keybindings.json` |
| Scripts | `~/Library/Application Scripts/com.coteditor.CotEditor/` (sandbox) | any executable, `.applescript`/`.scpt` | `$XDG_DATA_HOME/coteditor-clone/scripts/` |
| Settings archive (export/import) | user-chosen | `.cotsettings` | same |
| Snippets, file-drop rules, everything else | user defaults (`com.coteditor.CotEditor` plist) | — | GSettings or a JSON settings file |

The app bundles 14 themes (Anura, Anura (Dark), Classic, Classic (Dark), Dendrobates, Dendrobates (Dark), Kawazu, Lakritz, Mono, Note, Printen, Pulse, Resinifictrix, Resinifictrix (Dark)) and about 70 syntaxes. A clone can reuse those data files: they are JSON and Apache-2.0 licensed, so keep the licence notice.

### Global behaviours in one place

- **Launch** with nothing to restore: Settings → General "When nothing else is open" decides: create a new document (default), show the Open dialog, or do nothing. The same applies when the app is re-activated with no windows.
- **The empty untitled document is transient.** If it is still untouched when the user opens a file, the file replaces it in the same window.
- **Line endings**:
  - Each document has one line ending.
  - Typed and pasted text is normalised to it; Paste Exactly is the exception.
  - Mixed line endings found on open are reported (an alert that can be suppressed) and listed in the Warnings pane.
- **Encodings**: an ordered, user-customisable list. Auto-detection on open (see Document Lifecycle). Changing the encoding asks whether to **Convert** or **Reinterpret**.
- **Syntax**:
  - Chosen automatically from the file name, extension or interpreter (shebang).
  - Drives highlighting, outline, comment delimiters, completion words, indentation tokens and the Mode (General vs Code behaviour profile).
- **All edits are undoable** with descriptive undo names, for example "Sort Lines", "Shift Right", "Line Endings to LF", "Encoding to “UTF-8”", "Insert Snippet", "Insert Color Code", or the normalization form name.
- **Read-only ("Prevent Editing")** disables every editing command, the line-ending change and script output that would modify the text. Status bar shows a pencil-slash badge.


## Main Menu

Source basis: `Storyboards/Base.lproj/Main.storyboard` (static items and their **default** shortcuts), `AppDelegate.prepareMainMenu()` (dynamic submenus), the `validateUserInterfaceItem` implementations (dynamic titles/check states), `MainMenu.xcstrings` (English strings created in code).

### Conventions used below

- Shortcuts are written in macOS notation: ⌘ Command, ⌥ Option, ⌃ Control, ⇧ Shift; ↑ ↓ ← → arrow keys. In the storyboard an **upper-case letter key implies Shift** (e.g. key `N` + ⌘ = ⇧⌘N); this is already resolved below.
- Suggested Linux mapping: ⌘ → Ctrl, ⌃ → Super or Ctrl+Alt (avoid colliding with ⌘ mappings), ⌥ → Alt. A clone should keep the *relative* structure (same letters) where possible. Every menu shortcut is user-customizable (Settings → Shortcuts), and these storyboard values are the "defaults" the Shortcuts pane restores to.
- **Alternate items** (marked *alt*): shown in place of the preceding item while ⌥ is held. GTK has no native alternates — put them as normal items directly below, or in a submenu.
- **Section headers** (marked *header*): non-clickable bold small caption inside a menu (macOS 14+ "section header"). GTK4 `GMenu` sections support a `label` attribute — use that.
- Every item has an SF Symbol icon on macOS 26 (listed in brackets where present) — optional for a clone.
- "———" = separator.
- Menu titles shown in the macOS menu bar: **CotEditor · File · Edit · Format · View · Text · Find · Window · Script (shown as an icon, SF Symbol `applescript.fill`) · Help**.

### CotEditor (application) menu

| Item | Shortcut | Description |
|---|---|---|
| About CotEditor | — | Opens the About panel (small borderless panel: app icon, name, version, credits/licenses). |
| Check for Updates… | — | Only in the non-App Store (Sparkle) build; inserted at index 1, right after About. Linux: omit (distro/Flatpak updates). |
| Donate to CotEditor… [mug] | — | Opens the Donation panel (App Store tip jar). **[macOS/App Store-only]** — omit or link to a donation web page. |
| ——— | | |
| Settings… [gear] | ⌘, | Opens the Settings window (see Settings Window section). |
| ——— | | |
| Services ▸ | — | System Services submenu. **[macOS-only]** — omit. |
| ——— | | |
| Hide CotEditor | ⌘H | **[macOS-only]** |
| Hide Others | ⌥⌘H | **[macOS-only]** |
| Show All | — | **[macOS-only]** |
| ——— | | |
| Quit CotEditor | ⌘Q | Quits (with save prompts as described in Document Lifecycle). |

Linux: in a libadwaita app this content goes into the primary (hamburger) menu: Settings (Ctrl+,), Keyboard Shortcuts, About, Quit.

### File

| Item | Shortcut | Description |
|---|---|---|
| New | ⌘N | New untitled text document; opens as tab or window according to the user's tabbing preference. |
| New Window [macwindow.badge.plus] | ⇧⌘N | New untitled document, forced into a new separate window. |
| New Tab [plus.square.on.square] | ⌘T | New untitled document, forced into a tab of the frontmost window. |
| ——— | | |
| Open… | ⌘O | Open dialog (files **and folders**; a folder opens a folder window with file browser). Has the encoding accessory (see Open/Save Accessories). |
| Open Recent ▸ | — | System recent-documents list, then ——— and **Clear Menu**. Linux: keep a recent list (GtkRecentManager or own list). |
| ——— | | |
| Close | ⌘W | Closes the current tab/window (save prompt if needed). |
| Save | ⌘S | Saves; first save shows the Save dialog. |
| Save As… | ⇧⌘S | Save dialog (with encoding/line-ending/extension accessory). (macOS also shows "Duplicate"/"Rename…"/"Move To…" when autosave-in-place is on — standard NSDocument items.) |
| Revert to Saved | — | Reverts to the last saved file. With autosave-in-place on macOS this becomes a "Revert To ▸ Last Saved / Browse All Versions…" submenu; **Versions browser is [macOS-only]**. |
| ——— | | |
| Show in Finder [finder] | — | Reveals the document file in the file manager. Linux: "Show in Files" via `org.freedesktop.FileManager1.ShowItems`. Disabled for untitled docs. |
| Copy File Path [document.on.document] | *alt* (⌥) | Copies the document's absolute path to the clipboard (remote docs: the remote path). |
| Share… [square.and.arrow.up] | — | System share sheet with the file. **[macOS-only]** — omit or use a portal. |
| ——— | | |
| Page Setup… | ⇧⌘P | Page setup dialog. Linux: GtkPageSetupUnixDialog. |
| Print… | ⌘P | Print dialog with CotEditor's print-options accessory (see Print). |

### Edit

| Item | Shortcut | Description |
|---|---|---|
| Undo | ⌘Z | Undo; title shows the action name ("Undo Sort Lines", "Undo Shift Right"…). One undo stack per document (shared by split editors). |
| Redo | ⇧⌘Z | Redo. |
| ——— | | |
| Cut | ⌘X | |
| Copy | ⌘C | Multiple selections are copied joined by the document line ending, with per-selection line counts kept privately so a later multi-cursor paste can redistribute them. |
| Copy as Rich Text | ⇧⌘C | Copies selections as RTF including the syntax-highlight foreground colours and the editor font. Disabled when nothing is selected. (Linux: put `text/html` or `text/rtf` on the clipboard.) |
| Paste | ⌘V | Normal paste: **line endings in the pasted text are converted to the document's line ending**. If there are several insertion points and the clipboard is a single line, it is inserted at every caret; if it came from a multi-selection copy with as many groups, each caret gets its own piece. |
| Paste Exactly [document.on.clipboard] | *alt* ⌥⌘V | Pastes plain text without any modification (no line-ending normalization). |
| Delete | — | Deletes the selection. |
| Select ▸ [character.textbox] | | |
| &nbsp;&nbsp;Select All | ⌘A | |
| &nbsp;&nbsp;Select Line | ⇧⌘A | Selects the whole logical line(s) containing the selection; with multiple carets, every caret's line. |
| &nbsp;&nbsp;Select Word | ⌘D | With an empty selection: selects the word at each caret. With a selection: **adds the next occurrence** of the last selected text as an additional selection (wraps from the top, skipping already-selected occurrences; does nothing if none). Scrolls to it. |
| &nbsp;&nbsp;Select Enclosing Symbols | — | Selects the nearest pair of `()`, `{}`, `[]` or `<>` enclosing the selection, **including the two symbols**. Quotes are not considered; no-op if none. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Select Column Up | ⌃⇧↑ | Adds an insertion point on the line above the first selection, at the same visual column (column/rectangular editing). In vertical layout the title becomes "Select Column Right" and key ⌃⇧→. |
| &nbsp;&nbsp;Select Column Down | ⌃⇧↓ | Adds an insertion point below the last selection. Vertical layout: "Select Column Left", ⌃⇧←. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Split Selection by Lines [line.3.horizontal] | ⌃⇧L | Turns each selection into one selection per line (line contents, without line ending) → multi-cursor. |
| ——— | | |
| Complete | — | Shows the word-completion list at the caret (sources per Mode settings: document words, syntax keywords, standard words). macOS default trigger is ⌥⎋ / F5 via the text system. |
| ——— | | |
| Insert Backslash [character.cursor.ibeam] | ⌘¥ | Types `\` at every caret (for JIS keyboards). Low priority on Linux. |
| Insert Yen Mark | ⌘Y | Types `¥` at every caret. |
| Insert by Unicode Code Point… | — | Opens the Unicode input popover (see Unicode Input). |
| Insert from iPhone or iPad ▸ | — | Continuity Camera / sketch / Live Text. **[macOS-only]** — omit. |
| ——— | | |
| Spelling and Grammar ▸ | | System spell checker. Linux: libspelling (GtkSourceView) provides equivalent. |
| &nbsp;&nbsp;Show Spelling and Grammar | ⌘: | Opens the spelling panel. |
| &nbsp;&nbsp;Check Document Now | ⌘; | Jumps to next misspelling. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Check Spelling While Typing | — | Toggle (per Mode default, see Settings → Mode). |
| &nbsp;&nbsp;Check Grammar With Spelling | — | Toggle. **[macOS-only]** grammar. |
| &nbsp;&nbsp;Correct Spelling Automatically | — | Toggle. |
| Substitutions ▸ | | macOS text-system substitutions. |
| &nbsp;&nbsp;Replace Quotes | — | Converts straight quotes in the selection to smart quotes. |
| &nbsp;&nbsp;Straighten Quotes | — | Converts curly quotes `‘ ’ ‚ ‛` → `'` and `“ ” „ ‟` → `"` in the selection (enabled only with a non-empty selection). Implemented by CotEditor. |
| &nbsp;&nbsp;Replace Dashes | — | Converts `--` to dashes in the selection. |
| &nbsp;&nbsp;Replace Text | — | Applies the user's system text replacements. **[macOS-only]** |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Show Substitutions | — | System substitutions panel. **[macOS-only]** |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Smart Copy/Paste | — | Toggle (smart space handling). |
| &nbsp;&nbsp;Smart Quotes | — | Toggle (per Mode default). |
| &nbsp;&nbsp;Smart Dashes | — | Toggle (per Mode default). |
| &nbsp;&nbsp;Text Replacement | — | Toggle. **[macOS-only]** |
| Speech ▸ | | **[macOS-only]** (Start Speaking / Stop Speaking) — omit or use speech-dispatcher. |
| *(system-appended)* AutoFill, Start Dictation…, Emoji & Symbols | — | macOS adds these automatically at the end of Edit. **[macOS-only]**; for Emoji use the GTK emoji chooser (Ctrl+.). |

### Format

| Item | Shortcut | Description |
|---|---|---|
| Line Endings ▸ [return] | | Dynamic. Items: **macOS / Unix (LF)**, **Classic Mac OS (CR)**, **Windows (CRLF)**; hidden ⌥-alternates **Unix Next Line (NEL)**, **Unix Line Separator (LS)**, **Unix Paragraph Separator (PS)**. Check mark on the document's line ending. Choosing converts every line ending in the document (undo name "Line Endings to LF"). Disabled when read-only. |
| Text Encoding ▸ [arrow.2.squarepath] | | Dynamic: the user's encoding list (Settings → Format → encoding list, with its separators). If the document's encoding isn't in the list, it is inserted at the top followed by ———. Then ——— and **Customize Encodings List…** [square.and.pencil] (opens Settings → Format and the encoding list sheet). Check mark on the current encoding. Choosing one runs the Convert/Reinterpret flow (see Status Bar → encoding popup). |
| Syntax ▸ [curlybraces] | | Dynamic: **None**, ———, then all syntax names (alphabetical; bundled + user). Syntaxes hidden via "Customize Syntax Menu" are hidden unless current. Check mark on the current syntax. Choosing sets the document's syntax (re-highlights, updates outline, comment delimiters, completion words, Mode). |
| Reset Syntax Analysis [arrow.clockwise] | ⌥⌘R | Discards and re-runs syntax highlighting/outline parsing for the whole document. |
| ——— | | |
| Font ▸ [textformat] | | |
| &nbsp;&nbsp;Show Fonts | — | Opens the system font panel to change the editor font of this window. Linux: GtkFontDialog. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Font Type | *header* | |
| &nbsp;&nbsp;Standard | — | Radio: switch this window's editor to the user's standard (proportional) font from Settings → Appearance. Checked when the current font equals it (and it differs from the monospaced one). |
| &nbsp;&nbsp;Monospaced | — | Radio: switch to the user's monospaced font. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Bigger [textformat.size.larger] | ⌘+ | Zoom in the focused editor (text scale ×1.1; it's a view scale, not a font-size change). Also used by Console and Find results. |
| &nbsp;&nbsp;Smaller [textformat.size.smaller] | ⌘− | Zoom out. |
| &nbsp;&nbsp;Reset to Default [textformat.size] | ⌘0 | Reset zoom to 100 %. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Antialias | — | Toggle text antialiasing for this window's editors (check mark). |
| &nbsp;&nbsp;Ligatures | — | Toggle standard ligatures for this window's editors (check mark). |
| Theme ▸ [paintbrush] | | Dynamic: all theme names; check mark on this window's theme. Choosing applies the theme to this document window only (the default theme is set in Settings → Appearance). |
| ——— | | |
| Tab Width ▸ [tab.forward] | | |
| &nbsp;&nbsp;2 / 4 / 8 | — | Radio: set this window's tab width. |
| &nbsp;&nbsp;Custom… | — | Sheet with a number field to enter any tab width (see Custom Tab Width sheet). |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Use Spaces for Indentation | — | Toggle soft tabs (Tab key inserts spaces) for this window. |
| Show Invisibles [paragraphsign] | — | Toggle; title flips to **Hide Invisibles**. Disabled with tooltip "To show invisible characters, set them in Window settings" if all invisible kinds are off in Settings → Window. Per window. |
| Show Indent Guides [text.indentguides] | — | Toggle; title flips to **Hide Indent Guides**. Per window. |
| Show Page Guide [text.pageguide] | — | Toggle; title flips to **Hide Page Guide**. Per window. |
| ——— | | |
| Writing Direction ▸ [arrow.left.arrow.right] | | |
| &nbsp;&nbsp;Orientation | *header* | |
| &nbsp;&nbsp;Horizontal [text.alignleft] | — | Radio (default checked). |
| &nbsp;&nbsp;Vertical [text.vertical] | — | Radio: vertical (top-to-bottom, right-to-left columns) CJK layout. A GTK clone may omit/disable. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Direction | *header* | |
| &nbsp;&nbsp;Left to Right [text.alignleft] | — | Radio (default checked). |
| &nbsp;&nbsp;Right to Left [text.alignright] | — | Radio; also forces horizontal orientation. Shift Left/Right swap meaning in RTL. |
| Wrap Lines [text.append] | — | Toggle soft wrap; title flips to **Unwrap Lines**. Per window. |
| ——— | | |
| Prevent Editing [pencil.slash] | — | Makes the document read-only (title flips to **Allow Editing** [pencil]). Status bar shows the not-editable badge. |

### View

| Item | Shortcut | Description |
|---|---|---|
| Show Tab Bar | ⇧⌘T | System window-tab bar toggle (title flips Hide/Show). Linux: show/hide `AdwTabBar` (autohide off/on). |
| Hide Toolbar | ⌥⌘T | System toolbar toggle (flips Show/Hide). Linux: toggle header-bar extra widgets; optional. |
| Customize Toolbar… | — | System toolbar customization palette. **[macOS-only]** — Linux: a simple dialog with checkboxes of toolbar items, or omit. |
| ——— | | |
| Show Quick Actions… [filemenu.and.selection] | ⌘K | Opens/closes the Quick Actions command bar (see Command Bar). |
| ——— | | |
| Sidebar ▸ [sidebar.leading] | | Only enabled in folder windows; otherwise disabled with tooltip "The sidebar is only available when a folder is opened as a document." |
| &nbsp;&nbsp;File Browser [folder] | ⌃⌘1 | Opens the sidebar on the File Browser pane and moves focus into it (check mark when visible). |
| &nbsp;&nbsp;Folder Find [magnifyingglass] | ⌃⌘2 | Opens the sidebar on the Folder Find pane and focuses it. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Show Sidebar | ⌃⌘S | Toggle (flips to **Hide Sidebar**). |
| Inspector ▸ [sidebar.trailing] | | |
| &nbsp;&nbsp;Document Inspector [document] | — | Opens the inspector on the Document pane (check mark when that pane is visible). |
| &nbsp;&nbsp;Outline [list.bullet.indent] | — | Opens the inspector on the Outline pane. |
| &nbsp;&nbsp;Warnings [exclamationmark.triangle] | — | Opens the inspector on the Warnings pane. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Show Inspector [sidebar.trailing] | ⌘I | Toggle (flips to **Hide Inspector**). Per window. |
| ——— | | |
| Show Navigation Bar [inset.filled.topthird.rectangle] | — | Toggle (flips to **Hide Navigation Bar**). **Global** for all windows. |
| Show Line Numbers [inset.filled.leadingthird.rectangle] | — | Toggle (flips to **Hide Line Numbers**). Per window. |
| Show Status Bar [inset.filled.bottomthird.rectangle] | — | Toggle (flips to **Hide Status Bar**). **Global** for all windows. |
| ——— | | |
| Move Focus to Next Editor [inset.filled.bottomhalf.tophalf.rectangle] | ⌃\` | Cycles keyboard focus to the next split editor (wraps). From sidebar/inspector, returns focus to the editor. |
| Move Focus to Previous Editor | *alt* ⌃⇧\` | Reverse. |
| Split Editor [rectangle.split.1x2] | — | Adds another editor of the same document below/beside the focused one (max 4, beeps beyond); icon follows orientation. |
| Close Split Editor [rectangle] | — | Closes the focused split editor (disabled with only one). |
| Stack Editors Vertically [rectangle.split.2x1] | — | Toggles split orientation; title flips to **Stack Editors Horizontally** [rectangle.split.1x2]. Saved globally as the default for new windows. |
| ——— | | |
| Enter Full Screen | ⌃⌘F | System full screen (flips to Exit Full Screen). Linux: F11 / `gtk_window_fullscreen`. |

### Text

| Item | Shortcut | Description |
|---|---|---|
| Insert Snippet ▸ [text.viewfinder] | | Dynamic: general snippets (no scope) first, ———, then a *header* with the current syntax name and the snippets scoped to it, ———, then **Edit Snippets…** [square.and.pencil] (opens Settings → Snippets). Each item shows its user shortcut; a scoped snippet's shortcut wins over a general one with the same shortcut. |
| Comment ▸ [text.commentout] | | Delimiters come from the current syntax. |
| &nbsp;&nbsp;Toggle Comment | ⌘/ | If **all** selected lines are already commented → uncomment; otherwise comment out by **line** (placed after the common indentation when Settings → Edit "Insert comment delimiters after indent" is on, the default), inline style preferred over block. The menu title dynamically reads **Comment Out** or **Uncomment** accordingly. Disabled if the syntax has no comment delimiters. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Inline Comment | — | Inserts the first inline delimiter (e.g. `//`) at the start of each **selected line segment** (selection-based, not whole line). Enabled only if the syntax has inline delimiters. |
| &nbsp;&nbsp;Block Comment | — | Wraps each selection in the first block delimiter pair (e.g. `/* … */`). Enabled only if the syntax has block delimiters. |
| &nbsp;&nbsp;Uncomment | — | Removes inline or block delimiters (block only when the selection exactly fits a block). Enabled if any part can be uncommented. |
| Indentation ▸ [increase.indent] | | |
| &nbsp;&nbsp;Shift Right [increase.indent] | ⌘] | Prepends one indent unit (tab, or N spaces when soft tabs are on) to every line touched by any selection (in RTL it outdents). |
| &nbsp;&nbsp;Shift Left [decrease.indent] | ⌘[ | Removes one leading tab, or up to N leading spaces, from every touched line; no-op if nothing to remove. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Convert Indentation to Spaces [tab.forward.split] | — | In the lines of non-empty selections (or the whole document if nothing is selected), replaces each leading tab with tab-width spaces. |
| &nbsp;&nbsp;Convert Indentation to Tabs [tab.forward] | — | Replaces each leading run of tab-width spaces with a tab. |
| Surround Selection With ▸ [ellipsis.curlybraces] | | Wraps **every** selection (also empty carets → inserts the pair) and keeps the inner text selected. |
| &nbsp;&nbsp;Single Quotes | ⌘' | `'…'` |
| &nbsp;&nbsp;Double Quotes | ⇧⌘' (⌘") | `"…"` |
| &nbsp;&nbsp;Parentheses | ⇧⌘9 (⌘( ) | `(…)` |
| &nbsp;&nbsp;Braces | — | `{…}` |
| &nbsp;&nbsp;Brackets | — | `[…]` |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Custom… | ⇧⌘L | Sheet asking for begin/end strings (remembers the last pair per window). |
| Trim Trailing Whitespace [eraser] | — | Removes trailing spaces/tabs at all line ends of the whole document (whitespace-only lines are skipped unless Settings → Edit "Including whitespace-only lines" is on). |
| ——— | | |
| Lines ▸ [line.3.horizontal] | | |
| &nbsp;&nbsp;Move Up [arrow.up.to.line] | ⌃⌘↑ | Moves all lines touched by the selections up by one line (selections move along). Beeps if any selection is on the first line. |
| &nbsp;&nbsp;Move Down [arrow.down.to.line] | ⌃⌘↓ | Moves them down; beeps at the last line. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Sort [arrow.up.arrow.down] | — | Sorts the lines covered by the **first** selection (whole document if nothing selected) ascending by **localized string comparison** of the line contents. |
| &nbsp;&nbsp;Sort by Pattern… | — | Opens the Sort by Pattern sheet (sort by column/regex/CSV field, numeric, descending, etc.). |
| &nbsp;&nbsp;Reverse | — | Reverses line order (same range rule as Sort). |
| &nbsp;&nbsp;Shuffle | — | Randomises line order (same range rule). |
| &nbsp;&nbsp;Delete Duplicates [eraser] | — | Deletes later duplicate lines (exact match), keeping the first occurrence, among the lines touched by selections (whole document if nothing selected). |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;Duplicate Line [plus.square.on.square] | ⇧⌘D | Inserts a copy of the touched line(s) right below; selections move to the copy. A line ending is added if the last line has none. |
| &nbsp;&nbsp;Delete Line [eraser] | — | Deletes every line touched by a selection; caret goes to the start of where the line was. |
| &nbsp;&nbsp;Join Lines [plus.square.dashed] | ⇧⌘J | With non-empty selections: inside each selection, every line break plus its surrounding whitespace becomes one space. With only carets: joins each caret's line with the next one the same way. |
| ——— | | |
| Transformations ▸ [textformat.characters] | | All apply to every selection; **with an empty selection the word at the caret is selected first** and transformed. |
| &nbsp;&nbsp;Make Upper Case | — | Locale-aware upper case. |
| &nbsp;&nbsp;Make Lower Case | — | Locale-aware lower case. |
| &nbsp;&nbsp;Capitalize | — | Locale-aware capitalization of each word. |
| &nbsp;&nbsp;Convert to Full Width | — | All half-width characters that have full-width forms (ASCII, half-width katakana…) → full-width (ICU Fullwidth-Halfwidth transform, reversed). |
| &nbsp;&nbsp;Convert to Half Width | — | Full-width → half-width (ICU transform). |
| &nbsp;&nbsp;Convert Alphanumeric Text to Full Width | — | Only U+0021–U+007E → U+FF01–U+FF5E (shift +0xFEE0). |
| &nbsp;&nbsp;Convert Alphanumeric Text to Half Width | — | Reverse of the above. |
| &nbsp;&nbsp;——— | | |
| &nbsp;&nbsp;URL Encode [percent] | — | Percent-encodes everything except `A–Z a–z 0–9 - . _ ~` (non-ASCII letters are encoded too); existing valid `%XX` runs are not double-encoded. |
| &nbsp;&nbsp;URL Decode | — | Removes percent encoding (no change if invalid). |
| Unicode Normalizations ▸ [arrow.2.squarepath] | | Items (tooltip = description): **NFD** (Canonical Decomposition), **NFC** (Canonical Decomposition, followed by Canonical Composition), **NFKD** (Compatibility Decomposition), **NFKC** (Compatibility Decomposition, followed by Canonical Composition), **NFKC Case-Fold** (Applying NFKC, case folding, and removal of default-ignorable code points), ———, **Modified NFD** (Unofficial NFD-based normalization form used in HFS+), **Modified NFC** (Unofficial NFC-based normalization form corresponding to Modified NFD). Applies to selections (word at caret if empty); undo name = form name. |
| ——— | | |
| Inspect Character [character.bubble] | ⌥⌘I | Shows the Character Inspector popover for the single selected character (also inserted as the second item (index 1) of the editor context menu when exactly one character is selected). |
| Advanced Character Count… [numbers.rectangle] | — | Toggles the floating Advanced Character Count panel. |
| Edit Color Code… [eyedropper.halffull] | ⌃⌘H | Opens the color code panel (colour picker + code field; if the selected text parses as a colour code it is loaded; **Insert** replaces the selection with the code). |

### Find

All Find items act on the focused editor and use the Find panel's current strings and options (details in Find & Replace Panel).

| Item | Shortcut | Description |
|---|---|---|
| Find… [text.page.badge.magnifyingglass] | ⌘F | Shows the Find & Replace panel and focuses the find field (selects its content). |
| Find Next | ⌘G | Selects the next match (wraps if "Wrap search around"). |
| Find Previous | ⇧⌘G | Selects the previous match. |
| Find All | ⇧⌘F | Highlights all matches and shows the result table in the Find panel. |
| Select All Find Matches | — | Turns every match into a selection (multi-cursor). |
| ——— | | |
| Use Selection for Find [text.page.badge.magnifyingglass] | ⌘E | Copies the selected text into the find field (and turns Regular Expression off). |
| Use Selection for Replace | ⇧⌘E | Copies the selected text into the replace field. |
| Jump to Selection | ⌘J | Scrolls the editor so the selection is centered/visible. |
| ——— | | |
| Replace [text.page.badge.magnifyingglass] | — | Replaces the current match (selection) only. |
| Replace and Find Next | ⌥⌘G | Replaces, then selects the next match. |
| Replace All | — | Replaces all matches (in selection if "In Selection"). |
| Multiple Replace ▸ | | Dynamic: one item per saved replacement definition (runs it on the document), ———, **Manage Multiple Replace…** [square.and.pencil] (opens the Multiple Replace window). |
| ——— | | |
| Highlight All [highlighter] | ⇧⌘H | Highlights every match in the editor without selecting. |
| Remove Highlights | ⌃⇧⌘H | Clears those highlights. |
| ——— | | |
| Open Outline Menu [filemenu.and.selection] | — | Shows the navigation bar (if hidden) and pops open its outline menu. Disabled if no outline. |
| Select Previous Outline Item [text.line.magnify] | ⌥⌘↑ | Same as the navigation bar ⌃ button. |
| Select Next Outline Item | ⌥⌘↓ | Same as the navigation bar ⌄ button. |
| ——— | | |
| Go to Line… [number] | ⌘L | Opens the Go To Line sheet (input `N` or `N:M`). |

### Window

| Item | Shortcut | Description |
|---|---|---|
| Minimize | ⌘M | **[macOS-only]** (Linux: rarely used; can call `gtk_window_minimize`). |
| Zoom | — | Toggles between the user-defined window size and the previous size. |
| Keep on Top [pin] | — | Toggles floating window level (check mark). Linux/Wayland: no client-side "always on top" — **[limited on Linux]**; X11 only via `_NET_WM_STATE_ABOVE`, otherwise omit. |
| ——— | | |
| Bring All to Front | — | **[macOS-only]** |
| ——— | | |
| Console [apple.terminal] | — | Opens the Console panel (script output/errors). |
| *(system-appended)* tab items and window list | | macOS automatically adds Show Previous Tab (⌃⇧⇥), Show Next Tab (⌃⇥), Move Tab to New Window, Merge All Windows, and the list of open windows. Linux: implement Ctrl+PageUp/PageDown / Ctrl+Tab via AdwTabView; window list is optional. |

### Script

Shown in the menu bar as an icon only (SF Symbol `applescript.fill`; Linux: a "Scripts" menu or a header-bar menu button with a scroll icon). Contents are generated from the user scripts folder: script items (and sub-folders as submenus, `-` items as separators), then ———, then **Open Scripts Folder** [folder]. ⌥-click (or ⌥ + choosing) a script item opens the script file in CotEditor instead of running it (tooltip "Option-click to open script in editor."). Full rules in the Scripts Menu section.

### Help

| Item | Shortcut | Description |
|---|---|---|
| CotEditor Help | ⌘? | Opens the bundled Help Book. Linux: open local HTML help or the online manual (F1). |
| CotEditor Scripting Guide ▸ | | |
| &nbsp;&nbsp;About Scripting [book.pages] | — | Help page about scripting. |
| &nbsp;&nbsp;AppleScript Dictionary [character.book.closed] | — | Opens the AppleScript dictionary in Script Editor. **[macOS-only]** |
| ——— | | |
| What's New in CotEditor [sparkles] | — | Opens the What's New panel (also shown automatically once after upgrading). |
| Release Notes | — | Help page with release notes. |
| ——— | | |
| Create Issue Report [paperplane] | — | Opens a **new untitled document** (syntax Markdown) prefilled with an issue-report template (environment info), for the user to post. |
| Report Issue [paperplane] | *alt* (⌥) | Opens the GitHub issue tracker in the web browser. |
| CotEditor Web Site [arrow.forward.circle] | — | Opens coteditor.com. |

### Dock menu **[macOS-only]**

- **New Document** — activates the app and creates a new document. Linux: provide a "New Window" desktop action in the `.desktop` file.

### Menu-state rules worth copying

- Toggle items whose title flips (Show/Hide …, Wrap/Unwrap Lines, Prevent/Allow Editing, Comment Out/Uncomment, Stack Editors Vertically/Horizontally) rather than showing a check mark. GTK `GMenu` cannot change labels easily at runtime; use stateful boolean actions with a fixed label ("Invisibles", "Line Wrapping"…) or rebuild the menu model on state change.
- Radio-like groups show a check mark on the current value: line ending, encoding, syntax, theme, tab width, font type, orientation, direction, inspector pane, sidebar pane.
- Editing commands are disabled when the document is read-only.
- Text-editing commands that work on lines operate on **all** selections (multi-cursor aware) except Sort / Reverse / Shuffle / Sort by Pattern, which only use the **first** selection (or the whole document if it is empty).


## Document Window Layout

Source basis: `DocumentWindowController`, `DocumentWindow`, `WindowContentViewController`, `ContentViewController`, `DocumentViewController`, `EditorViewController`, `EditorTextViewController` (CotEditor `main`, shallow clone 2026-11-03). Strings are the English values from `Document.xcstrings` / `MainMenu.xcstrings`.

### Two kinds of document window

| Kind | Opened when | Toolbar identifier | Frame autosave name | Split autosave name |
|---|---|---|---|---|
| **File window** | a single file is opened / a new untitled document is created | `Document` | `Document` | `WindowContentSplitView` |
| **Folder window** ("directory document") | a folder is opened (Open dialog allows choosing folders; dropping a folder on the app) | `DirectoryDocument` | `DirectoryDocument` | `DirectoryWindowContentSplitView` |

Folder windows never take part in window tabbing (tabbing disallowed); file windows do (see Document Lifecycle → tabs).

### Overall structure (horizontal split view, left → right)

```
┌──────────── title bar + unified toolbar (full-size content view) ─────────────┐
│ [Sidebar]        │ [Content area]                              │ [Inspector]   │
│ (folder windows  │  ┌ Navigation bar (24 px) ───────────────┐  │ (collapsed by │
│  only)           │  │ editor 1                              │  │  default,     │
│  segmented tabs: │  ├ Navigation bar ───────────────────────┤  │  min width    │
│  File Browser /  │  │ editor 2 (split, up to 4)             │  │  200 px)      │
│  Folder Find     │  └───────────────────────────────────────┘  │ segmented     │
│                  │  Status bar (bottom accessory, full width   │ tabs: Doc /   │
│                  │  of the content column)                     │ Outline /     │
│                  │                                             │ Warnings      │
└──────────────────┴─────────────────────────────────────────────┴───────────────┘
```

- Root is a 3-item split view: **sidebar** (only exists in folder windows), **content**, **inspector**. Divider positions and collapse state are autosaved per window kind (names above).
- Initial content size 640 × 720 pt. If the user set *Window size* width/height in Settings → Window, new windows use that size (each dimension independently; empty = "Auto" = last saved frame). Zoom (green button / double-click title) toggles to that user size too.
- New windows cascade from the frontmost visible document window.
- **Sidebar**: shown expanded whenever the folder window appears; its pane selector (segmented control with two icon segments) sits at the top of the sidebar as a top-aligned accessory. Panes: `File Browser` (SF Symbol `folder`) and `Folder Find` (SF Symbol `magnifyingglass`). Tooltips are the pane names. Hidden temporarily while the Versions browser is shown, then restored.
- **Content area** shows exactly one of:
  - a text editor stack (for text documents),
  - a **file preview** (for non-text files opened inside a folder window — see File Browser Sidebar),
  - the **"No document selected"** empty state (folder window with no file selected): centered, large outlined `document` SF Symbol (64 pt tall, light weight) above the text "No document selected" (16 pt, medium), on a thick material background.
- **Status bar** is attached to the bottom of the content column (not under sidebar/inspector). Toggling it animates.
- **Inspector**: collapsed initially; min thickness 200 pt, no max; no titlebar separator. Its pane selector is a large segmented control (icon only) at the top.

### Window title

- Title = document display name. If another open window shows a file with the same name, the title gets ` — <first distinguishing parent folder name>` appended (EM DASH with spaces) so they can be told apart; recomputed whenever any document's URL changes or a window opens.
- Folder windows: title = folder name; **subtitle** = name of the file currently shown in the content area (empty if none).
- Remote (rmate) documents: no proxy icon; subtitle "Remote" while connected, "Disconnected" after the connection drops.
- The proxy icon / title menu (rename, move, lock, versions) is standard NSDocument behaviour **[macOS-only]** — Linux: put Rename…/Move To… into the File menu or a title popover.
- Window **tab** shows a small 4 pt filled circle (tertiary colour, 8 pt padding) as accessory when the document has unsaved changes; tooltip "Document has unsaved changes". (Close-button "edited" dot is the standard macOS indicator.)

### Window-level appearance options (from settings, applied live)

- **Appearance** (Settings → Appearance "document appearance"): Default (follow system) / Light / Dark — forces the window's appearance.
- **Window opacity** (`windowAlpha`, 0.2–1.0, default 1.0): when < 1 the window becomes non-opaque and its background uses the editor background colour with that alpha. Ignored while in full screen or Versions browser (forced opaque), restored afterwards. Not applied in "viewing mode" (Versions browser).
- **Keep on Top** (Window menu item / toolbar item): toggles the window's level between normal and floating. State saved with window restoration.

### Focus

- When a window appears, the (first) editor text view becomes first responder.
- View → "Move Focus to Next Editor" (⌃`) / "Move Focus to Previous Editor" (⌃⇧`): cycles focus among split editors, wrapping around; from the sidebar/inspector it puts focus back into the editor.

### Window tabbing keyboard

- In a tabbed window, ⌘1 … ⌘8 select tab 1–8 and ⌘9 selects the last tab — but only if that key equivalent isn't claimed by a main-menu item first. (Folder windows: not applicable.)

---

## Toolbar

`NSToolbar`, user-customizable, configuration auto-saved per identifier (`Document` / `DirectoryDocument`). Item identifiers are prefixed `com.coteditor.CotEditor.ToolbarItem.`. Images marked *(custom)* are custom symbol assets in the app bundle; a clone should draw/borrow equivalents (e.g. from Adwaita/GNOME icon set).

### Default set and order

- **File window**: `Syntax` popup · *inspector tracking separator* · *flexible space* · `Toggle Inspector`.
- **Folder window**: *flexible space* · `Toggle Sidebar` · *sidebar tracking separator* · `Back/Forward` · `Syntax` popup · *inspector tracking separator* · *flexible space* · `Toggle Inspector`.

Immovable items: inspector tracking separator, flexible space, Toggle Inspector (and in folder windows also flexible space, Toggle Sidebar, sidebar tracking separator). (Layout intent: sidebar toggle at the trailing edge of the sidebar column, syntax popup left in the content column, inspector toggle far right.)

### Allowed items (customization palette), in palette order

| Identifier suffix | Label (palette label) | Tooltip | Icon | Behaviour |
|---|---|---|---|---|
| *(system)* toggleSidebar | Sidebar | — | system | Folder windows only. Shows/hides sidebar. |
| `documentHistory` | **Back/Forward** (group); subitems **Back**, **Forward** | "Go back or forward in document history"; subitems "Go to the previous document" / "Go to the next document" | `chevron.backward`, `chevron.forward` | Folder windows only. Navigational group (two momentary buttons). Click = go back/forward one entry in the folder window's document history. **Long-press/click-and-hold** on either shows a menu of the history entries in that direction (file name + 16 px file-type icon); choosing one jumps there. Disabled when no entry in that direction. In text-only (overflow) mode it becomes a menu "Back/Forward" ▸ Back, Forward. |
| `syntaxStyle` | **Syntax** | "Change syntax" | — (popup button) | Toolbar-style popup button listing: `None`, separator, section header **Recently Used** (most recent first, max 6 by default) + separator, then all syntax names alphabetically (hidden syntaxes are hidden unless they are the current one or in the recent list). The current syntax is checked/selected. If the current document's syntax no longer exists (deleted), a separator + section header **Deleted** + the orphaned name are inserted at the top and selected. Disabled when the window shows a non-text file. Overflow menu form = "Syntax" ▸ same menu. |
| *(system)* toggleInspector | Inspector | — | system | Shows/hides inspector. |
| `textSize` | **Text Size** (group); **Smaller**, **Bigger** | "Change text size"; "Decrease text size" / "Increase text size" | `textformat.size.smaller`, `textformat.size.larger` | Momentary 2-button group. Zoom out/in the focused editor by factor 1.1. |
| `writingDirection` | **Writing Direction**; **Left to Right**, **Right to Left** | "Change writing direction"; "Left to right" / "Right to left" | `text.alignleft`, `text.alignright` | Select-one segmented group reflecting current direction; no segment selected when text is vertical. Choosing RTL also switches orientation back to horizontal. |
| `textOrientation` | **Text Orientation**; **Horizontal**, **Vertical** | "Switch text orientation"; "Horizontal" / "Vertical" | `text.alignleft`, *(custom)* text.vertical | Select-one segmented group. Vertical = vertical (tategaki) layout. **[mostly CJK; a GTK clone may omit vertical layout or mark it unsupported]** |
| `indent` | **Indent**; **Shift Left**, **Shift Right** | "Indent selection"; "Shift lines to the left" / "Shift lines to the right" | `decrease.indent`, `increase.indent` | Momentary 2-button group; order reversed in RTL UI. Outdent/indent selected lines. |
| `comment` | **Comment** | "Comment-out or uncomment selection" | *(custom)* text.commentout | Toggle comment on selection (same as Text → Comment). |
| `tabStyle` | **Tab Style** | off: "Use spaces for indentation"; on: "Use tabs for indentation" | *(custom)* `tab.forward` (off) / `tab.forward.split` (on) | Menu toolbar item: **click** toggles "auto expand tabs" (spaces vs. tab); its **menu** (arrow/long-press) contains: section header **Tab Width**, items `2`, `4`, `8` (checked = current), `Custom…` (opens Custom Tab Width sheet), separator, `Use Spaces for Indentation` (checkmark = state). |
| `wrapLines` | **Line Wrapping** | off: "Wrap lines"; on: "Unwrap lines" | *(custom)* `text.wrap` (off) / `text.wrap.slash` (on) | Toggles soft wrap for this window. Icon shows the action that the click will perform. |
| `invisibles` | **Invisibles** | off: "Show invisible characters"; on: "Hide invisible characters" | `paragraphsign` (off) / *(custom)* paragraphsign.slash (on) | Toggles invisibles for this window. **Disabled** (tooltip "To show invisible characters, set them in Window settings") when every invisible-character type is turned off in Settings → Window. |
| `indentGuides` | **Indent Guides** | off: "Show indent guide lines"; on: "Hide indent guide lines" | *(custom)* text.indentguides (on state drawn with palette colours tertiary/label) | Toggles indent guides. |
| `keepOnTop` | **Keep on Top** | "Keep the window always on top" | `pin` (off) / `pin.slash` (on) | Toggles floating window level. |
| `editable` | **Edit** | editable: "Prevent the document from being edited"; read-only: "Allow editing the document" | `pencil.slash` while editable / `pencil` while read-only | Toggles the document's editable flag (read-only mode). |
| `spellCheck` | **Spell Check** | "Show spelling and grammar" | `textformat.abc.dottedunderline` | Opens the Spelling and Grammar panel. Linux: a spell-check dialog / libspelling. |
| `emojiAndSymbols` | **Emoji & Symbols** | "Show Emoji & Symbols" | *(custom)* emoji | Opens the system character palette **[macOS-only]** — Linux: GTK emoji chooser (`GtkEmojiChooser`) or insert-Unicode dialog. |
| `fonts` | **Fonts** | "Change Font" | `textformat` | Menu button (no indicator): section header **Font Type**, `Standard`, `Monospaced` (checked = current type), separator, `Show Fonts` (opens font panel; Linux: `GtkFontDialog`). |
| `find` | **Find** | "Show Find & Replace" | `magnifyingglass` | Opens the Find & Replace panel. |
| *(system)* print | Print | — | system | Print document. |
| *(system)* writingTools | Writing Tools | — | system | Only if Apple Intelligence Writing Tools exist **[macOS-only]** — omit. |
| `share` | Share (system label) | "Share document file" | system share | System share picker with the document file **[macOS-only]** — Linux: portal "Open With…" / send-to, or omit. |
| *(system)* space / flexibleSpace | | | | spacers |

Notes:
- "Statable" items swap their **image** according to state (the image depicts the action the next click performs, e.g. wrap-on shows "wrap.slash").
- Toolbar item validation follows the focused editor in the window.
- Toolbar customization was reset by a migration in 7.1.0 → defaults above are the current intended look.
- Touch Bar items exist (text size slider etc.) **[macOS-only]** — ignore.

---

## Navigation Bar (Outline Navigator)

One navigation bar sits **above each split editor** (each editor pane has its own). Height 24 pt, window-background colour, small control size, horizontal padding 4 pt. Visibility is global (`showNavigationBar`, default **on**), toggled by View → "Show Navigation Bar" / "Hide Navigation Bar"; animates collapse. Its splitter is not draggable.

Layout left → right:

1. **Close split editor** button — `xmark`, icon-only, 18 pt wide; tooltip "Close split editor" (a11y "Close Split Editor"). Visible (fades in) only when the editor stack has more than one editor; otherwise invisible and non-interactive (keeps its space).
2. **Divider** — shown only when the close button is visible *and* the outline has items.
3. **Outline navigation** (only when the current syntax produced outline items; nothing at all if the item list is empty):
   - **Previous / Next** buttons, icon-only, medium weight, 18 pt each:
     - Horizontal text: `chevron.up` = Previous, `chevron.down` = Next.
     - Vertical text: order swapped and icons `chevron.left` (Next) then `chevron.right` (Previous).
     - Tooltips "Jump to previous outline item" / "Jump to next outline item"; a11y "Previous Outline Item" / "Next Outline Item".
     - **Exact behaviour**: separators are skipped. *Previous* = the last non-separator item that lies before the item containing the selection start (i.e. "current item" = last item whose location ≤ selection start; previous = the one before it). *Next* = if the selection end is before the first item → the first item; otherwise the first non-separator item after the item containing the selection end. Selecting an item **selects its range** (the heading text) in the editor and scrolls to it. Each button is disabled when no such item exists.
     - Same actions in the Find menu: "Select Previous Outline Item" (⌥⌘↑) and "Select Next Outline Item" (⌥⌘↓).
   - **Outline popup** (borderless small popup button, a11y "Outline Menu"): shows the current outline item title (leading indentation stripped in the button title). Menu lists all outline items in document order: each with an icon for its kind, indentation level (from the syntax's level or literal leading whitespace), title; *container* and *mark* kinds bold; separator items as menu separators. A hidden placeholder item "No Selection" (secondary colour) is shown as the title when the caret is before the first item. The selected item follows the caret: last non-separator item whose location ≤ selection start (debounced 50 ms; not updated during IME composition or while an edit is pending). Choosing an item selects its range in the editor.
   - Find → "Open Outline Menu": un-hides the navigation bar if hidden and pops the outline menu open (disabled when there are no items).
   - **Kind icons** (SF Symbol, hierarchical colour): container `cube.square.fill` (custom) blue; value `dot.square.fill` green; function `f.cursive.square.fill` (custom) orange; title `tag.square.fill` teal; heading without level `list.bullet.square.fill` (custom) indigo; heading level n `n.square.fill` indigo; mark `flag.fill` red; separator `minus` gray.
   - While outline parsing has not finished and has taken > 1 s, the text "Extracting Outline…" (secondary colour) is shown instead.
4. Spacer.
5. **Split editor** button — custom `split.add` image, icon-only, 18 pt; tooltip "Split editor" (a11y "Split Editor"). Rotated −90° when editors are stacked side by side. Click = split this editor (see Split Editor). **Right-click / context menu**: one item, "Stack Editors Vertically" (`rectangle.split.2x1`) when currently stacked top/bottom, or "Stack Editors Horizontally" (`rectangle.split.1x2`) when side-by-side → toggles orientation.

---

## Editor View

Each editor = scroll view + text view + line-number ruler + navigation bar. All split editors share one text buffer (one undo manager per document).

### Geometry & rendering

- Text container inset: 4 pt horizontal, 6 pt vertical (vertical inset grows with overscroll).
- **Line height** = font line height × `lineHeight` multiple (default **1.3**, Settings → Appearance); all lines get the same fixed height even with fallback fonts; glyphs vertically centred in the line.
- Tab stops: tab width × width of a space in the current font; tab width default **4** (min 1).
- Fixed-pitch fonts get kerning 0 (no proportional kerning).
- Antialiasing and ligatures per font type (Settings → Appearance: standard font antialias on / ligatures on; monospaced antialias on / ligatures off). Format → Font → "Antialias" / "Ligatures" toggles for this window only.
- **Fonts**: two font types, *Standard* and *Monospaced*; which one a document uses comes from its Mode (per-syntax mode settings) and can be switched per window via Format → Font or the Fonts toolbar item. Defaults: system font / system monospaced font.
- Theme: default theme "Anura"; when the system switches light/dark, the window switches to the theme's light/dark counterpart (e.g. "Anura (Dark)") unless *pin theme appearance* is set. Theme name is saved with window state.
- Non-contiguous layout only for documents > 5,000,000 UTF-16 units (and never for vertical text).

### Line numbers (gutter)

- Shown by default (`showLineNumbers` = on); View → "Show Line Numbers"/"Hide Line Numbers" toggles per window.
- Vertical ruler on the leading side (moves to the right for RTL writing direction). Horizontal ruler at the top for vertical text.
- Width = max(digits(total lines), 3) + 2 digit-widths, minimum 32 pt; grows as line count grows.
- Number font: system "line number" monospaced-digit font at the **editor font size × zoom scale**. Colour = editor text colour at 40 % opacity (normal), 100 % for lines that contain a selection/caret, drawn in a medium-weight font. Under the system "Increase Contrast" setting, all numbers are drawn at 100 % and selected ones in semibold.
- One number per **logical** line (on the first fragment of a wrapped line; wrapped continuation fragments are unnumbered). The extra empty last line after a trailing newline is numbered too.
- Vertical text: tick mark on every line; numbers only for line 1, every 5th line, the last line, and selected lines.
- **Mouse on the gutter** (works like many code editors):
  - Click: selects the whole clicked line (including its line ending); focus moves to the text view.
  - Drag: extends selection line-wise between the press line and current line; auto-scrolls.
  - ⇧-click: if the click is inside the current selection, shrinks it (click in upper half → keep from click to end; lower half → keep from start to end of clicked line); otherwise extends the selection to include the clicked line.
  - ⌘-click/drag: adds the line range as an additional selection; if the clicked line range lies inside an existing selection, that selection is split around it (toggle-off).
  - Scroll wheel over the gutter scrolls the editor.

### Invisible characters

- Global toggle default **off** (`showInvisibles`); per-window toggle via Format → "Show Invisibles"/"Hide Invisibles" and the Invisibles toolbar item. Which kinds are drawn comes from Settings → Window (defaults: new line **on**, tab **on**, space **off**, other whitespaces **on**, control characters **on**).
- Symbols (drawn as vector paths in the theme's *invisibles* colour, not as text): new line (LF, CR, CRLF, NEL, LS, PS) `↩`; tab `→`; space `·`; no-break spaces (U+00A0, U+2007, U+202F) `·̂`; full-width space U+3000 `□`; other spaces U+2000–200A, U+205F `⹀`; control & format characters (C0/C1, ZWSP, ZWNJ, WJ, BOM, bidi controls, invisible operators, interlinear annotation) `�`.
- Control characters, when shown, are laid out with a blank box of fixed width so the replacement glyph has room.
- **Line endings that differ from the document's line ending are drawn in red** (system red) instead of the invisibles colour.

### Indent guides

- Default off; toggle per window. Vertical hairlines at every tab-width columns of leading indentation, in the invisibles colour.

### Page guide

- Default off; column default **80** (Settings → Window). Vertical 1 px line at x = space-width × column + inset + line padding + 2 pt; colour = theme text colour at 20 % alpha (50 % with Increase Contrast). Mirrored for RTL. Per window toggle: Format → "Show Page Guide"/"Hide Page Guide".

### Current line highlight

- Setting `highlightCurrentLine` default **off**. When on, the full-width line rectangle(s) of every logical line containing a caret/selection are filled with the theme's line-highlight colour (all carets in multi-cursor mode).

### Matching brace highlight

- `highlightBraces` default **on**: after the caret moves (not while dragging), if the character just before an empty caret is one of `( ) [ ] { }`, its counterpart within the visible area flashes with the system "find indicator" (yellow pop-up bounce). Linux: briefly draw a rounded yellow highlight that fades.

### Selection instance highlight

- `highlightSelectionInstance` default **on**, delay **0.5 s**: when exactly one non-empty selection is a whole word (regex `\A\b\w.*\w\b\z`, ≥ 2 letters) and no multi-cursor, all other whole-word occurrences get a **rounded background** in the theme highlight colour at 30 % alpha. Cleared immediately on any selection change or edit.

### Wrap & hanging indent

- Soft wrap default **on** (`wrapLines`); per window toggle Format → "Wrap Lines"/"Unwrap Lines". Wrap width tracks the view width.
- A line never breaks immediately after its leading indentation.
- Hanging indent (`enablesHangingIndent` default **on**, `hangingIndentWidth` default **0** spaces): wrapped continuation lines are indented to the line's own leading indentation plus N extra space-widths.

### Overscroll

- `overscrollRate` default **0** (Settings → Window "Overscroll"). Adds blank space = rate × (visible height − one line), split equally above the first line and below the last line (implemented as vertical text-container inset, minimum 6 pt).

### Zoom (text size)

- Per editor scale (not changing the font setting): View/Format → Font → "Bigger" (⌘+), "Smaller" (⌘−), "Reset to Default" (⌘0): ×1.1 / ÷1.1 / 1.0, keeping the visible area anchored. Pinch to zoom with a "sticky" detent at 100 % (haptic tick); two-finger double-tap toggles 100 % ↔ 150 %. Range = scroll view magnification limits (≈ 0.25–4×). Line numbers scale too.

### Selection, word selection, double-click

- Double-click selects a word; additional characters are treated as word separators (CotEditor-specific word boundaries).
- **Double-click on a quote character** that begins/ends a syntax-highlighted string selects the whole string token; otherwise double-click on a quote selects the quoted span (pair matched within ±50,000 chars); double-click on a bracket `( ) [ ] { } < >` selects from that bracket to its matching counterpart (inclusive).
- Edit → Select → "Select Word" (with empty selection: selects the word at each caret; with a selection: **adds the next occurrence** of the selected text as an extra selection, wrapping to the top; like ⌘D in other editors).
- "Select Line"/"Select Paragraph" (logical line) works per caret in multi-cursor mode.
- "Select Enclosing Symbols": selects the nearest enclosing `()`, `{}`, `[]` or `<>` pair, including the two symbols (quotes are not considered).

### Multi-cursor / multiple selection rules

- **⌘-click** (single click, no drag) adds an insertion point at the click, or removes an existing one there.
- **⌥-drag** = rectangular (column) selection: produces one caret/selection per line in the rectangle.
- Edit → Select → "Select Column Up" (⌃⇧↑) / "Select Column Down" (⌃⇧↓): add a caret one visual line above the first / below the last selection (same x position).
- Edit → Select → "Split Selection by Lines" (⌃⇧L — key equivalent uppercase `L` + ⌃): turns each selection into one selection per line (line contents without line ending).
- Typing, deleting, Tab/indent, cursor movement, and most commands apply to every caret.
- Empty carets are kept separately from real selections; overlapping/touching carets merge.
- **Esc** exits multi-cursor mode, keeping only the first caret and scrolling to it.
- **Cut** with multiple selections leaves a caret at each place.
- **Paste**: if the clipboard holds a single-line string and there are multiple carets → same text inserted at every caret. If the clipboard came from a multi-selection copy (private pasteboard type recording how many lines each piece had) and there are multiple carets → pieces are distributed to the carets in order (extra pieces go to the last caret; missing ones become empty).
- Additional carets are drawn as real insertion-point indicators (blinking), hidden when the window isn't key.

### Cursor movement specials

- **Smart Home** (⌘← / Home key binding to *move to beginning of line*): first jumps to the start of the current *visual* (wrapped) line fragment; if already there (or not wrapped), jumps to the first non-whitespace character after indentation; if already there, jumps to the very start of the logical line. ⌘⇧← extends selection the same way.
- ⌘→: end of the visual line.
- ⌥←/⌥→: word-wise with CotEditor word boundaries, repeated presses continue.
- ⌥⇧↑ / ⌥⇧↓: extend to beginning/end of the logical line, repeatedly to previous/next lines.
- Emacs bindings (^A, ^E, ^B, ^F, ^N, ^P, ^K, ^D, ^T) operate on logical lines and every caret.
- ↑/↓ with multiple carets move all carets visually.
- CRLF is treated as one unit by arrows/deletion even when the document's line ending is not CRLF.
- Arrow-key scrolling scrolls by the minimum distance (line by line) instead of re-centring.

### Typing behaviours (driven by Edit settings & Mode)

- **Auto-indent** (`autoIndent` default on): Return inserts the document's line ending (never a literal different one) then copies the previous line's indentation; with syntax block delimiters (indent tokens) it adds one level after an opener, and typing a closer at the start of a line outdents (smart outdent).
- **Tab key**: if "Indent with Tab key" (`indentWithTabKey`, default off) and there is a selection → indent selected lines (⇧Tab outdents). Else if "auto expand tabs" (soft tabs; `autoExpandTab` default off) → inserts spaces up to the next tab stop at every caret. Backspace in leading soft-tab whitespace deletes back to the previous tab stop.
- **Indent style detection** (`detectsIndentStyle` default on): on open, if the file's indentation is detectably tabs or spaces, the window's soft-tab flag is set accordingly (width is not detected).
- **Auto-close brackets/quotes** (Mode option "automatic symbol balancing"): typing `(`, `[`, `{` (and the syntax's single-character string/character quote delimiters) inserts the pair and puts the caret between, only if the next character is not alphanumeric (and for symmetric quotes, the previous character isn't alphanumeric either). With a selection, typing an opener **wraps the selection**. Typing the closer right before an auto-inserted closer just steps over it. Backspace between an empty auto pair deletes both.
- **Trim trailing whitespace** (`autoTrimsTrailingWhitespace` default off): 3 s after the last edit (keeping the line being edited) and on every explicit save; `trimsWhitespaceOnlyLines` default off.
- **Completion**: ⌥Esc/F5 standard completion; automatic completion (Mode option) pops up after ≥ 3 word characters typed, 0.25 s debounce; candidates from the syntax's completion words / document words per Mode settings.
- **Line ending normalisation**: any text inserted/pasted containing line endings different from the document's line ending is converted to the document's line ending.
- **File drop**: dropping files into the editor inserts text from the "file drop" snippet settings matching extension + syntax (defaults: Markdown image/link, HTML img/script/a, CSS `url()`); otherwise inserts the absolute path (one per line). `.textClipping` files insert their text **[macOS-only]**.
- Smart quotes/dashes, spell checking, text replacement come from Mode options (macOS text system services — Linux: implement or omit).

### Context menu (editor)

Standard text context menu with changes: inserts **"Inspect Character"** (`character.bubble`) at position 1 when exactly one character is selected (opens the Character Inspector popover); adds **"Copy as Rich Text"** after Copy; adds **"Select All"** after Paste; adds **"Straighten Quotes"** to the Substitutions submenu; removes the system Font and Layout Orientation submenus. Services, Speech, Writing Tools, Continuity Camera ("Insert from iPhone" → Live Text popover) **[macOS-only]**.

### Other editor-attached UI

- **Go to Line** (Find → "Go to Line…", ⌘L) opens a sheet (see the accessory-views part of the spec).
- **Unicode input** (Edit menu) opens a popover anchored at the caret.
- **Character Inspector** popover anchored at the selected character (detachable).
- **Advanced Character Count**: Text → "Advanced Character Count…" first shows an options sheet, then floats a draggable counter overlay in the editor's top-right (top-left for RTL/vertical); the menu item then reads "Stop Advanced Character Count". Persisted with window state.
- **Incompatible characters** (when the Warnings pane is visible) are highlighted with an unemphasized-selection background colour in the editor.
- URLs are auto-linked when `autoLinkDetection` is on (default off).

---

## Split Editor

- View → "Split Editor" or the navigation bar split button: inserts a new editor **after** the editor that triggered it (or the focused one). The new editor copies font, antialias, ligature, theme, tab width, soft-tab flag, current syntax highlights, wrap, invisibles, orientation, page/indent guides, line numbers, writing direction; gets the same selection; both editors scroll to it; focus moves to the new editor.
- **Maximum 4 editors**; further requests beep.
- Each split has its own navigation bar, line numbers, scroll position, selection, zoom scale. Edits and undo are shared (same buffer).
- Orientation: one global setting (`splitViewVertical`, default **false** = editors stacked top-to-bottom). View → "Stack Editors Vertically" / "Stack Editors Horizontally" (title flips to the other option; icons `rectangle.split.2x1` / `rectangle.split.1x2`) or the split button's context menu toggles it for the window and saves it as the new default.
- Minimum editor thickness 100 pt; dividers draggable.
- "Close Split Editor" (View menu, enabled only if > 1 editor) or the navigation bar ✕ closes that editor; focus goes to the previous editor (or the last if the first was closed).
- Focus cycling: ⌃` / ⌃⇧` (wraps).
- Window-level toggles (wrap, invisibles, guides, line numbers, orientation, writing direction, tab width, soft tabs, font type, antialias, ligatures) apply to **all splits of that window**; zoom applies to the focused split only.

---

## Status Bar

Bottom bar of the content column. Visible by default (`showStatusArea` = on); View → "Show Status Bar"/"Hide Status Bar" is a **global** setting (all windows). Small control size, single line, monospaced digits, secondary text colour. Height 18 pt + 7 pt vertical padding (pre-macOS-27 style with a top divider); background transparent unless "prefers opaque bar background" (Settings → Appearance) is on, then window background colour.

Layout left → right:

1. *(Donation badge — coffee-mug icon toggle that pops "Thank you for your kind support!"; only for App Store tip-jar subscribers.)* **[macOS App Store only]** — omit.
2. **Not-editable badge**: `pencil.slash` icon, tooltip "The document is not editable.", appears (fade) only while the document is read-only.
3. **Editor counts** (text documents only), in this fixed order, each shown only if enabled in Settings → Window → "Status bar shows:":

| Item | Settings checkbox | Default | Format |
|---|---|---|---|
| Lines | "Line count" | **on** | `Lines: <total>` or `Lines: <total> (<selected>)` |
| Characters | "Character count" | **on** | `Characters: <total>` / `Characters: <total> (<selected>)` |
| Words | "Word count" | off | `Words: <total>` / `Words: <total> (<selected>)` |
| Location | "Location" | **on** | `Location: <n>` |
| Line | "Current line" | **on** | `Line: <n>` |
| Column | "Current column" | off | `Column: <n>` |

   - Label in secondary colour, value in primary colour; numbers use locale grouping (e.g. `4,512`). Unknown values show an en dash `–` in tertiary colour.
   - Items are separated by spacing; when space is short, trailing items are dropped (truncating stack).
   - **Counting rules** (exact):
     - *Characters* = number of user-perceived characters (grapheme clusters) in the whole text; selected = sum over **all** selection ranges.
     - *Lines* (total) = number of lines **not counting the empty line after a trailing line ending** (empty document = 0; "a\n" = 1; "a\nb" = 2). Selected = number of **distinct lines touched by non-empty selections** (empty carets count 0; two selections on the same line count once).
     - *Words* = locale-aware word enumeration; selected = sum over all selections.
     - The parenthesised `(n)` appears only when the selected count > 0. So with a caret only: `Lines: 195`; with one partial-line selection: `Lines: 195 (1)`.
     - *Location* = 0-based character (grapheme) offset of the **start of the first selection** from the document start.
     - *Line* = 1-based line number of the first selection's start.
     - *Column* = 0-based grapheme offset of the first selection's start from its line start.
   - Updates: content counts debounced 20 ms after edits; selection-dependent values debounced 200 ms; counting runs in the background; nothing is counted for items that are hidden (and nothing at all while the status bar is hidden — unless the Document inspector is visible, which forces all counts).
   - No click action on count items.
4. Spacer.
5. **File size**: byte-count formatted in "file" style (e.g. `12 KB`; zero not spelled out), tooltip "File size"; `–` for unsaved documents. Shown also for preview (non-text) documents.
6. Divider.
7. **Text encoding popup** (borderless small popup; tooltip "Text Encoding"): section header "Text Encoding", then the encoding list from Settings → Format (with separators; "Unicode (UTF-8) with BOM" automatically follows UTF-8). If the document's encoding is not in the list, it is shown first followed by a divider. **Choosing an encoding**: if the document is untitled or empty → change immediately. Otherwise an alert sheet "Text encoding change" / "Do you want to convert or reinterpret this document using “<encoding>”?" with buttons **Convert**, **Reinterpret**, **Cancel** (+ help button):
   - Convert: if the text can't be represented → error sheet "The document contains characters incompatible with “<enc>.”" / "Incompatible characters are either substituted or removed in saving. Do you want to change the text encoding and review the incompatible characters?" buttons **Change Encoding** / **Cancel**; Change Encoding converts and opens the Warnings inspector. Conversion is undoable ("Encoding to “X”").
   - Reinterpret: if unsaved changes → "The document has unsaved changes." / "Are you sure you want to discard your changes and reopen the document using “<enc>”?" **Cancel** / **Discard Changes** (destructive). Then re-reads the file with that encoding; failure → beep + "The document could not be reinterpreted using text encoding “X.”" / "The document may have been saved using a different text encoding, or it may not be a text file."
   - If cancelled, the popup reverts to the actual encoding.
8. Divider.
9. **Line ending popup** (borderless small popup, 48 pt wide; tooltip & a11y "Line Endings"): section header "Line Endings", items `LF`, `CR`, `CRLF` (tooltips "macOS / Unix", "Classic Mac OS", "Windows"); `NEL`, `LS`, `PS` ("Unix Next Line", "Unix Line Separator", "Unix Paragraph Separator") are hidden alternates revealed while holding ⌥ (Linux: show them in a submenu "Other" or always). Choosing converts **all** line endings in the buffer (undoable "Line Endings to LF"); also allowed when choosing the current ending if the document has inconsistent endings (normalises them). Disabled when read-only.

Accessibility: the editor text view's accessibility help reads the enabled counts, e.g. "Lines: 195 (1 selected), Characters: …".

---

## Inspector

Right-hand panel. Toggle: View → Inspector → "Show Inspector"/"Hide Inspector" (⌘I in the storyboard; per window) and the toolbar inspector button. View → Inspector submenu also has "Document Inspector", "Outline", "Warnings" (radio-style checkmark on the visible pane; choosing one opens the inspector on that pane). The selected pane is remembered globally (`selectedInspectorPaneIndex`, default 0 = Document).

Pane selector: large icon-only segmented control at the top, tooltips = pane names:

| # | Pane | Icon |
|---|---|---|
| 0 | Document Inspector | `document` |
| 1 | Outline | `list.bullet.indent` |
| 2 | Warnings | `exclamationmark.triangle` |

Pane contents only observe/compute while that pane is visible.

### Document Inspector

Scrollable column (padding 12, spacing 8, small controls) of collapsible sections (disclosure groups, all expanded initially; header 12 pt semibold secondary colour). Each section is a grouped form of label → value rows; missing values show `–`.

1. **File**
   - **Created** — creation date, abbreviated date + short time.
   - **Modified** — modification date, same format.
   - **Size** — file byte count incl. exact bytes (e.g. "1 KB (1,024 bytes)").
   - **Tags** — Finder tags as coloured dot + name, wrapping, right-aligned **[macOS-only]** (Linux: omit or use `xdg` emblems/metadata).
   - **Permissions** — `644 (-rw-r--r--)` style: octal (3 digits, 4 with special bits) + symbolic incl. setuid/setgid/sticky (`s/S`, `t/T`).
   - **Owner** — owner account name.
   - **Full Path** — path without scheme (remote: the remote path), up to 5 lines, middle truncation, selectable, tooltip full path; followed by a small circular-filled `arrow.forward` button "Show in Finder" that reveals the file (Linux: open containing folder via `org.freedesktop.FileManager1.ShowItems`). File attributes are blank for remote documents.
2. **Text Settings** (text documents only)
   - **Encoding** — e.g. "Unicode (UTF-8)" / "Unicode (UTF-8) with BOM".
   - **Line Endings** — `LF` / `CRLF` …
   - **Mode** — the mode name ("General", "Code", or a syntax-specific mode name).
3. **Count** (text documents only), two groups:
   - Lines, Characters, Words — same format as status bar (`195 (1)`).
   - Location, Line, Column.
   While this pane is visible, all counts are computed regardless of status bar settings.
4. **Character** (text documents only)
   - When the selection is exactly one character: **Code Point** (or **Code Points** for multi-scalar clusters) — each scalar as a bordered chip `U+1F408`; for single-scalar characters also **Name** (Unicode name), **Block** (localized block name), **Category** (e.g. "Uppercase Letter (Lu)").
   - Otherwise: placeholder "Not selected" (tertiary), tooltip "Select a single character to show Unicode information."

### Outline

- Header row: "Outline" (12 pt semibold, secondary) + trailing **Options** menu button (`ellipsis.circle`, borderless, no indicator) with:
  - Toggle **Wrap Lines** (default off) — wraps long outline titles.
  - Section **Text Size**: **Bigger** (`textformat.size.larger`), **Smaller** (`textformat.size.smaller`, min = small system size), **Reset to Default** (`textformat.size`). Default font size = small system size (11 pt). ⌘+/⌘−/⌘0 also work while the list has focus.
- **List** (rounded 8 pt clip, no row separators) of the outline as a **tree**: items whose syntax gives a numeric level are nested by level (an item becomes a child of the nearest preceding item with a lower level); items with literal string indentation are top-level and show the indentation text. Each row: kind icon (larger scale; mark kind medium) + title; bold for container/mark kinds, italic if the syntax style says so. Separator items render as dividers (not selectable). Disclosure triangles; newly appearing parent nodes are expanded by default, user-collapsed nodes stay collapsed across re-parses.
- **Selection sync**: the row of the item containing the caret (last non-separator item with location ≤ caret) is selected automatically as the caret moves (not during IME composition or before the re-parse after an edit finishes). **Clicking a row** selects that item's range in the editor and **centres it** in the visible area.
- **Filter field** at the bottom (search field with recents autosaved as "OutlineSearch"): fuzzy "abbreviation" match — the filter's characters must appear in order, case-insensitive, anywhere in the title; matched characters get a find-highlight (yellow) background with near-black text. Parents of matching items stay visible; all matching branches are expanded while filtering. Empty result → centred "No Filter Results" (secondary).
- Empty (no outline in syntax) → empty list.

### Warnings

Vertical split (resizable) with two sections; padding 16/12/12/12.

**Incompatible Characters** (top, min height 60 pt / 120 pt when it has rows)
- Header "Incompatible Characters".
- Status line:
  - "Scanning incompatible characters…" during the first scan,
  - "Found an incompatible character." / "Found <n> incompatible characters.",
  - "No issues found." (secondary) when none (also always when the encoding is a Unicode encoding or the document is empty — no scan is done).
- Table (bordered, rounded 8 pt, sortable columns): **Line** (right-aligned, 1-based), **Character** (the character; control/space/line/paragraph separators shown as `U+XXXX` in tertiary colour), **Converted** (what the character becomes in the target encoding, e.g. `?`, or empty if removed).
- Re-scans 0.3 s after edits and whenever the document encoding changes.
- While visible, all incompatible characters are highlighted in the editor (unemphasized selection colour); highlights cleared when hidden/cleared.
- Clicking a row selects that character in the editor and centres it.

**Inconsistent Line Endings** (bottom)
- Header "Inconsistent Line Endings".
- Status: "No issues found." or "Found a line ending other than LF." / "Found <n> line endings other than <LF>." (the document's line ending label).
- Table: **Line**, **Line Ending** (label, e.g. `CRLF`), sortable. Clicking a row selects that line ending in the editor and centres it.
- Updates live as the buffer changes.

**When the Warnings pane appears automatically** (the inspector opens and switches to Warnings):
- choosing **Review** in the "inconsistent line endings" alert on open;
- choosing **Review Incompatible Characters** in the lossy-save error;
- choosing **Change Encoding** in the lossy-conversion error (encoding change).
There is no badge on the inspector button; the pane is never forced open otherwise.

---

## File Browser Sidebar

Only in folder windows. First sidebar pane ("File Browser"); the second pane, "Folder Find", is specified in the find section.

### Tree

- Single-column outline view, no header, multiple selection, autosaved expansion state (`FileBrowserOutlineView`). Root = the opened folder (root itself not shown; its children are top level).
- Sort: **folders first**, then files, each in Finder-like "localized standard" order (natural number order, case-insensitive).
- `.DS_Store` and `.git` are never listed. Hidden files (dot-files / hidden flag) are listed only when "Show Hidden Files" is on (`fileBrowserShowsHiddenFiles`, default off, global); hidden items are drawn with 50 % icon opacity and tertiary text colour.
- Row: SF Symbol by kind — folder `folder`, generic document `document`, archive `zipper.page`, image `photo`, movie `film`, audio `music.note`, executable `apple.terminal.fill` — + filename; aliases/symlinks get a small arrow badge overlay (a11y "Alias"); Finder tag colour dots trailing **[macOS-only tags]**.
- **Single-click / selection of a single file** opens it in the window's content area (replacing the current one). Text-like files open as editable text documents; other files (images, media, archives, executables, binary plists, symlinks…) open as a **preview** (see below). A file whose default app is CotEditor, which matches a syntax by name/extension, or which has no extension also opens as text. Alias to a file opens its original if accessible.
- Selecting a folder does nothing; **double-click** toggles expansion.
- Previously shown documents are closed when another file is shown, **except edited ones**, which stay open in the background (so unsaved edits aren't lost); the window's close button shows "edited" while any member document has unsaved changes.
- If the file is already open in another window: error "The file “<name>” is already open in another window." / "To open it in this window, close the existing window first."; that other window is brought front.
- Renaming: standard in-place editing of the name cell (click on the name of an already-selected row; also triggered automatically after New File/New Folder) edits the filename (only if writable and the file isn't an edited open document). Validation errors: too long (> NAME_MAX bytes), empty, contains "/" or ":", contains a newline, name already exists. Renaming the current document re-detects its syntax from the new name. Tab inside the field inserts a tab rather than moving focus.
- Keyboard: **Space** = Quick Look **[macOS-only]** (Linux: open preview / sushi), **Delete** (Backspace) on a selection = Move to Trash (with confirmation), ⌘C copies the file URLs.
- External changes in the folder (files added/removed/renamed/moved) refresh the affected subtree automatically.

### Footer

- Thin separator above the footer, visible only when the list is scrolled/overflowing.
- **"+" menu button** (circular glass style; a11y "Add"): **New File** (`document.badge.plus`), **New Folder** (`folder.badge.plus`). Target = selected folder, or the parent folder of the selected file, or the root when nothing is selected; disabled if not writable or multiple items are selected.
  - New File creates an empty file named "Untitled" + the default syntax's first extension (Settings → Format default syntax; e.g. `Untitled.txt`), uniquified with a number ("Untitled 2.txt"); New Folder creates "untitled folder" (uniquified). The new row is selected and put into **rename editing** immediately; creating clears any filter.
- **Filter field** (search field, recents saved as "FileBrowserSearch"; a11y "File browser filter"): 0.5 s debounce; case-insensitive **substring** match on file names, searching recursively through the whole folder (respects hidden-file setting); a small spinner (a11y "Searching in folder…") shows inside the field while searching. Matching parts of names are drawn in primary label colour while the rest of filtered rows is secondary; ancestors of matches are shown and expanded. No match → centred "No Filter Results". Clearing the filter restores the previous expansion state and keeps the selection visible. Drag-and-drop is disabled while filtering.

### Context menu (on clicked row, or on the selection if the clicked row is selected)

In order:
1. **Show in Finder** (`finder`) — reveal in file manager. Hidden when no target.
2. —
3. **Open in New Window** (`macwindow.badge.plus`) — detaches the file into its own normal window (closing it here); for folders/folder aliases opens a new folder window. Hidden when no target.
4. **Open with External Editor** (`arrow.up.forward.square`) — system "Open With…" chooser **[macOS dialog]** (Linux: `GtkAppChooserDialog` / portal OpenURI with ask). Hidden unless at least one target is a file.
5. —
6. **Move to Trash** (`trash`) — confirmation sheet "Do you want to move “<name>” to the Trash?" (or "Do you want to move <n> selected items to the Trash?") / "This operation cannot be undone." buttons **Move to Trash** / **Cancel** (no default button). Open documents for those files (or inside trashed folders) are closed. Hidden when no target; disabled if none writable. (Linux: `g_file_trash`.)
7. **Duplicate** (`plus.square.on.square`) — copies as "<name> copy", "<name> copy 2"… next to the original. Only for a single target (not for folders while filtering).
8. —
9. **New File**, **New Folder** — as footer.
10. —
11. **Share…** (`square.and.arrow.up`) **[macOS-only]**.
12. —
13. **Show Hidden Files** (`eye`, checkmark state) — toggles the global setting.

Services menu for selected file URLs **[macOS-only]**.

### Drag and drop

- Rows can be dragged out (copy/move/delete-to-Trash supported). Dragging to the Trash in the Dock deletes them **[macOS]**.
- Drop target is always a folder (dropping on a file retargets to its parent; dropping between rows is not allowed — drop *on* items or on empty space = root). Rejected when destination not writable or the destination is one of the dragged items or inside them.
- Internal drag = **move** (⌥ forces copy); drags from outside = **copy**. Name collisions get a number suffix.

### Preview of non-text files (content area)

Centred on thick material: Quick Look preview (max 512 pt or the file's natural size) **[macOS-only]** (Linux: thumbnail via GdkPixbuf/GStreamer or a large file-type icon), filename (medium, secondary), buttons, and a grouped info form (max 400 pt wide):
- For aliases: **Show in Finder**, and **Open in New Window** (folder alias) or **Open Original** (file alias, opens target as text in this window).
- Otherwise: **Open with External Editor** split-menu button (primary click = open with default app; menu: default app with " (default)" suffix, separator, other capable apps sorted by name (with version when names collide), separator, **Other…** = choose an application) and **Open as Plain Text** (forces opening the file as text here).
- Info rows: **Kind** (localized type description, or "Unknown" italic); for links **Original** (target path); images **Dimensions** (`W × H` with hair spaces), **Image DPI** ("72 pixels/inch"), **Color space**, **Color profile**; movies **Dimensions**, **Duration**; audio **Duration** (m:ss or h:mm:ss).

### Document history (folder windows)

- Every file shown in the folder window is recorded (URL + "opened as plain text" flag + type). Navigating to a new file truncates forward history; re-showing the current entry replaces it. Entries whose file disappeared are skipped/removed.
- Back/Forward toolbar group (see Toolbar). The window restores its last shown file on relaunch (security-scoped bookmark) and selects it in the browser.

---

## Document Lifecycle

### Launch & "nothing open"

- Settings → General "When nothing else is open:" — **Create New Document** (default) / **Show Open Dialog** / **Do Nothing**; applied on launch and when the Dock icon is clicked with no visible windows.
- "Reopen windows from last session" (`quitAlwaysKeepsWindows`, default **on**) → window restoration (see below).
- **Transient untitled document**: the automatic untitled document created at launch/reopen is "transient". If the user then opens a file while that document is still empty and has no sheet, the file **replaces it in the same window** instead of opening a second window. Any edit, a second new document, or adding any document clears the transient flag.
- File → "New" (⌘N) obeys the tabbing preference; "New Window" (⌘⇧N) forces a separate window; "New Tab" (⌘T) forces a tab (enabled only when the current document is a text document). New untitled documents get the default syntax from Settings → Format (default "Plain Text") and the default line ending (Settings → Format, default LF) and default encoding (default UTF-8, no BOM).

### Opening a file

1. Folder → folder window (DirectoryDocument).
2. Safety checks (before reading), each presented as a recoverable error with **Open** / **Cancel**:
   - Binary-looking type (image, audio/video, archive conforming types) **and** no syntax matches the filename: "The file “<name>” doesn’t appear to be text data." / "The file appears to be <type description>.\n\nDo you really want to open the file?"
   - Larger than 200 MB (hidden default `largeFileAlertThreshold`): "The file “<name>” has a size of <size>." / "Opening such a large file can make the application slow or unresponsive.\n\nDo you really want to open the file?"
3. Read the bytes; decode with the **encoding detection order**:
   1. If the user picked a specific encoding in the Open dialog (or reinterpretation is requested) → that encoding only (failure = error "inapplicable encoding").
   2. **BOM**: UTF-8, UTF-16 BE/LE, UTF-32 BE/LE BOMs — used if the encoding family is in the candidate list.
   3. **Extended attribute** `com.apple.TextEncoding` saved by CotEditor/TextEdit (empty files just trust it) **[macOS xattr; Linux equivalent: `user.` xattr or skip]**.
   4. **Encoding declaration** in the content (if Settings → Format "refer to encoding declaration" `referToEncodingTag`, default **on**), scanned in the first 1024 bytes: CSS `@charset "…";` at file start, `coding:`/`encoding:`/`fileencoding=`/`coding=` in the first 2 lines (Python/Ruby/Emacs/Vim), HTML/XML `charset="…"` / `encoding="…"` (only if everything before it is ASCII); the earliest one wins; IANA name → encoding; accepted only if it is in the candidate list and decodes successfully.
   5. Try each encoding of the **user's encoding priority list** (Settings → Format; default starts UTF-8, Shift JIS, EUC-JP, …, Mac Roman, Windows Latin 1, Chinese, Korean, … ) in order; first that decodes wins. When reverting an already-open document, its current encoding is tried first.
   6. Nothing worked → error "unknown string encoding".
   - UTF-8 with BOM is remembered as "UTF-8 with BOM" and the BOM is written back on save.
4. **Line ending** = the most common line ending found in the file (keep default if the file has none). Mixed endings are kept as is in the buffer.
5. **Syntax** (first open only): from filename (exact filenames, extensions), then shebang/interpreter and content patterns of the syntax definitions; else default.
6. Vertical text orientation restored from the file's `com.coteditor.VerticalText` xattr (if `savesTextOrientation`, default on).
7. Window shown. If the file has **inconsistent line endings** (and neither the global "suppress inconsistent line endings" warning nor this file's suppression xattr is set, and not in Versions browser) → warning sheet:
   - Title "The document has inconsistent line endings."
   - Editable: "Do you want to convert all line endings to <LF>, the most common line ending in this document?" buttons **Convert**, **Review**, **Ignore**.
   - Read-only: "The most common line ending in this document is <LF>." buttons **Review**, **Ignore**.
   - Suppression checkbox "Don’t ask again for this document" → stores xattr `com.coteditor.AllowLineEndingInconsistency` on the file.
   - Convert = normalise all endings (undoable); Review = open Warnings inspector.
8. Open-dialog option "Open as read-only" → document opens non-editable.

### Editing state

- **Read-only / editable**: Format → "Prevent Editing" / "Allow Editing" toggles, toolbar "Edit". Non-editable documents: typing disabled, line-ending popup disabled, encoding change blocked with "The document is not editable." / "To make changes to the document, allow editing first.", status bar shows the pencil-slash badge. Saved with window state. (File-system read-only files are not auto-flagged; macOS "locked" files go through the standard NSDocument Unlock/Duplicate flow **[macOS-only]** — Linux: when the file isn't writable, show an info bar "File is read-only" with "Save As…".)
- Undo/redo shared across splits; saving breaks undo coalescing.

### Saving

- ⌘S Save / ⇧⌘S Save As (Duplicate/Rename/Move To/Revert in macOS File menu style). Save panel accessory: checkbox "Make file executable" (initially reflects the file's current user-x bit; applies chmod u+x on save, default 0644 base when new). The save panel suggests the syntax's first extension for untitled documents and allows any extension.
- Before writing: if the text can't be represented in the document encoding → error sheet "The document contains characters incompatible with “<enc>.”" / "Incompatible characters are either substituted or removed in saving. Do you want to save anyway?" buttons **Save Available Text** / **Review Incompatible Characters** / **Cancel**. "Save Available Text" sets a per-document "allow lossy saving" flag (cleared when the problem disappears or encoding changes); Review opens the Warnings inspector.
- On explicit save (not autosave) trailing whitespace is trimmed if that setting is on.
- Yen sign `¥` is converted to backslash when the encoding can't represent `¥` (Japanese encodings).
- Written atomically ("safely"); extended attributes written: encoding name (only if the file already had one or the user changed encoding), vertical-text flag, line-ending-inconsistency suppression flag.
- After **Save As**, syntax is re-detected from the new filename/content.
- Script hooks: `document opened` / `document saved` events are dispatched to user scripts.

### Auto-save, versions, backups

- Settings → General "Document save:" checkbox **"Enable Auto Save with Versions"** (default **on**; explanatory note: "A system feature that automatically overwrites your files while editing. Even if turned off, CotEditor covertly creates a backup in case it unexpectedly quits."). The value is read **once at launch**; changing it asks "The change will be applied first on the next launch." / "Do you want to restart CotEditor now?" **Restart Now** / **Later**.
- **On** (autosave in place): the document file is overwritten automatically about **5 s** after changes pause (autosave delay 5.0); untitled documents are autosaved to a hidden location; the Versions browser (File → Revert To → Browse All Versions) is available **[macOS-only — Linux: offer "Revert to Saved" only, optionally keep timestamped backup copies]**. The process enables "sudden termination" (quit without asking).
- **Off**: changes are periodically backed up elsewhere (`~/Library/Autosaved Information/<name> (<UUID>).<ext>` for files that have a URL) so they survive crashes, but the real file is only written on explicit Save; standard "Do you want to save…?" prompts on close/quit. In this mode an **empty** document never shows as edited (no dot/close prompt).
- Closing an untitled-or-draft document whose text is **empty** never prompts; its autosaved file (if any) is deleted.
- Closing with unautosaved changes that can't be encoded → custom alert: "Do you want to save the changes made to the document “<name>”?\n<lossy encoding description>" / "Your changes will be lost if you don’t save them. Incompatible characters are either substituted or removed in saving." buttons **Save Available Text**, **Don’t Save** (destructive), **Cancel**.

### External file changes

- The document observes its file (file presenter). Settings → General "When document is changed by another application:" — **Keep CotEditor’s version** (ignore) / **Ask how to resolve** (notify) / **Update to modified version** (revert) — default **Update to modified version**.
- Detection: when the file's modification date differs from the known one, the file is re-read and compared byte-for-byte with what CotEditor last read or wrote; if the content is identical (e.g. touched, or a cloud client adjusted the date) only the stored date is updated — no prompt.
- *Update* mode: reverts silently (even if there are unsaved changes in CotEditor). Selections are preserved (clamped) across revert; content-dependent alert sheets are dismissed first.
- *Ask* mode: sheet (critical style if another sheet is already attached, so it interrupts it):
  - Message "The file has been changed by another application." — or, when there are unsaved changes, "The file has been changed by another application. There are also unsaved changes in CotEditor."
  - Informative "Do you want to keep CotEditor’s version or update it to the modified version?"
  - Buttons **Keep CotEditor’s Version** / **Update** (Update = revert from disk). Never queued twice.
- Own saves don't trigger the alert (modification date is refreshed before saving).

### Closing & quitting

- Closing a folder window asks for each edited member document in turn (standard save prompt); cancelling any stops the close. All member documents close with the window.
- Quit: all documents follow the autosave mode above; with autosave-in-place on, windows are restored next launch with their unsaved state.

### Window restoration (state saved per window)

- Per document: editable flag, transient flag, vertical text, syntax, line-ending alert suppression, and — only if the text contains characters that can't be encoded — the full unencoded text and encoding; allows-lossy-saving flag.
- Per window/editor: line numbers, page guide, indent guides, invisibles, wrap, vertical orientation, soft tabs, writing direction, theme name (converted to light/dark equivalent unless pinned), advanced character counter visibility, tab width, window opacity, floating level, inspector pane, splitter positions; Handoff/user-activity stores selected ranges and restores them (with scroll).
- Folder windows restore the last shown file.

### Window tabbing

- Settings → Window "Open documents in tabs:" — **Respect System Setting (<current system value>)** (default) / **Never** (manual) / **In Full Screen** / **Always**. Opening multiple files at once in "In Full Screen" mode puts files after the first into tabs of the first new window. Native macOS tab bar, View → "Show Tab Bar", Window → Merge All Windows etc. **[macOS-native tabbing — Linux: use `AdwTabView`/`AdwTabBar` in one window per tab group, with the same preference values]**.

### Remote editing (rmate) — summary

- Optional rmate server (Settings → General "Remote editing", default off, default rmate port). Files sent from a remote shell open as **RemoteDocument** using a local working copy; Save sends the content back over the connection; window subtitle "Remote"/"Disconnected"; Copy Path copies the remote path; Move/Rename/Versions/Revert/Show in Finder disabled; not added to Recent Documents.
- On disconnect while edited: alert "The remote connection for “<name>” was disconnected." / "Changes can no longer be saved remotely. To keep your changes, save the document on this Mac." **Save As…** / **Later**. Closing a disconnected edited remote doc: "Save the changes to “<name>” on this Mac?" / "The remote connection is disconnected. Your changes have not been sent." **Save As…** / **Cancel** / **Don’t Save**. Limits: errors "The remote file is too large to open." ("Remote editing supports files up to <size>."), "Some remote files couldn’t be opened." ("A single rmate command can open up to <n> files at a time."), port errors "The port number is invalid." / "The port is already in use." / "Enter a port number from 1 to 65535."

---

## Open/Save/Print Accessories

### Open dialog accessory (always disclosed)

Form below the standard file chooser; the dialog also allows choosing **folders**.

| Control | Type | Options / default |
|---|---|---|
| **Text encoding:** | popup | **Automatic** (default; = detection order above), divider, then the user's encoding list (with dividers as in Settings). Disabled when only folders are selected. |
| **Open as read-only** | checkbox | off. Disabled (and reset to off) when only folders are selected. |
| **Show invisible files** | checkbox | off; toggles showing hidden files and treating packages as folders in the dialog (Linux: `GtkFileChooser` "show-hidden"). |

The chosen encoding/read-only options apply to all files opened from that dialog session (and are discarded afterwards).

### Save dialog accessory

| Control | Type | Default |
|---|---|---|
| **Make file executable** | checkbox | reflects the file's current user-executable bit (off for new files) |

Options apply only if the save completes; cancelled panels discard them.

### Print

Print uses the current editor's settings: font family of the focused editor at the print font size, tab width, ligatures, orientation (vertical text prints vertically), writing direction, syntax highlighting, line height, hanging indent, current invisible-kind settings. URLs in the printed PDF are linked. Margins: top 56 pt, left 24 pt, bottom 56 pt, right 24 pt (page not vertically centred). The standard panel shows paper size, orientation, scaling, and "Selection only" when there is exactly one non-empty selection. Accessory pane ("CotEditor" options, summarised in the panel's summary list as Font Size / Color / Print Backgrounds / Line Number / Invisibles / Header / Footer):

| Label | Control | Options | Default |
|---|---|---|---|
| **Color** | popup | **Black and White**, section header **Theme**, then all theme names | last used print theme, else the current default theme for the appearance |
| **Print Backgrounds** | switch | on/off | off |
| **Print Line Numbers** | switch | on/off | follows the window's current line-number visibility |
| **Print Invisibles** | switch | on/off | follows the window's current invisibles visibility |
| **Print Headers and Footers** | switch | on/off; when off the header/footer rows are disabled | **on** |
| **Header** — primary | popup + 3-segment alignment control | content: None, Syntax Name, Document Name, File Path, Print Date, Last Modified Date, Page Number; alignment segments **Align Left** (`arrow.left.to.line`), **Center** (`arrow.right.and.line.vertical.and.arrow.left`), **Align Right** (`arrow.right.to.line`) | **File Path**, Align Left |
| **Header** — secondary | popup + alignment | same | **Print Date**, Align Right |
| **Footer** — primary | popup + alignment | same | **None**, Center |
| **Footer** — secondary | popup + alignment | same | **Page Number**, Center |
| **Font Size** | number field + stepper + "pt" | 4–100 | system font size (13 pt) |

Header/footer text: 9 pt; content strings — Document Name; File Path (with `~` abbreviation; falls back to the name for unsaved); Syntax Name; "Printed on <date time>"; "Last modified on <date time>" (uses *now* when there are unsaved changes; `–` if unknown); page number as a bare number. If primary and secondary are left/right aligned they share one line (tab-separated); otherwise two lines. Line numbers are printed with 10 pt padding from the text.

Page Setup (⇧⌘P) is the standard dialog **[macOS]** — Linux: `GtkPageSetupUnixDialog`; the whole accessory maps naturally onto a custom tab in `GtkPrintUnixDialog` (`gtk_print_operation` "create-custom-widget").


## Text Commands: Common Semantics

The item-by-item list is in **Main Menu → Edit / Format / Text**. This section collects the rules that are shared by many commands, so a clone can implement them once.

### Which text a command acts on

| Command group | Target |
|---|---|
| Case transformations, width conversions, URL encode/decode, Unicode normalization | Every selection. **If the (primary) selection is empty, the word at each caret is selected first**, then transformed. Result stays selected. |
| Straighten Quotes | Every selection. Disabled when the selection is empty. |
| Surround (quotes, (), {}, [], Custom…) | Every selection; an empty caret gets an empty pair with the caret between. The inner text stays selected. |
| Comment / Uncomment | Every selection. Toggle Comment works on **whole lines** (or after common indent, see below). Inline/Block Comment work on the exact selection. |
| Shift Left / Right | Every line touched by any selection, including a trailing empty last line. |
| Convert Indentation to Spaces/Tabs | Lines of **non-empty** selections. If there are none, the whole document. Only leading indentation is converted. |
| Move Up/Down, Duplicate Line, Delete Line, Delete Duplicates | Every line touched by any selection (multi-cursor aware). Delete Duplicates uses the whole document if nothing is selected. |
| Join Lines | Inside non-empty selections, or (only carets) each caret's line with the next one. |
| Sort, Sort by Pattern…, Reverse, Shuffle | Lines covered by the **first** selection only; the **whole document** if that selection is empty. When the last line has no line ending, it is added as needed so that lines can be reordered. |
| Trim Trailing Whitespace | The whole document. When run by the auto-trim setting (not via the menu), lines that hold a caret are left untouched. |
| Select Word (⌘D), Select Line, Split Selection by Lines, Select Column Up/Down | Operate on the selection set itself (multi-cursor creation). |

### Comment details

- Delimiters come from the syntax: a list of inline delimiters (`//`, `#`…) and block pairs (`/* */`, `<!-- -->`…). The **first** inline and the **first** block pair are used for commenting. All of them are recognised for uncommenting.
- **Toggle Comment (⌘/)**:
  - Decision: if *every* targeted line is already commented, it uncomments; otherwise it comments out.
  - Commenting out prefers the inline style; the block style is used only when the syntax has no inline delimiter.
  - The menu item title changes live between "Comment Out" and "Uncomment".
- **Settings → Edit → Comment:** has two options used here:
  - **"Add a space to comment delimiters"** (default on): adds one space after an inline or begin delimiter and before an end delimiter, and expects that space when uncommenting.
  - **"Insert comment delimiters after indent"** (default on): Toggle Comment puts the delimiter at the minimal common indentation column of the targeted lines instead of column 0.
- Block uncomment only matches when a selection exactly encloses a block (`/* … */`). Nestable block delimiters are balanced-checked.
- Toolbar "Comment" button = Toggle Comment.

### Indentation details

- An indent unit is one tab, or *tab width* spaces when "Use Spaces for Indentation" (soft tabs) is on.
- Shift Right prepends one unit to each touched line (it does not align to tab stops).
- Shift Left removes one leading tab, or up to *tab width* leading spaces.
- The selection is adjusted so that the same text stays selected.
- In right-to-left writing direction, Shift Left and Shift Right swap behaviour.
- Toolbar "Indent" group = Shift Left / Shift Right.
- Tab-key behaviour, auto-indent, smart indent after `{`/`:` tokens and backspace-to-unindent are described under Editor View → Typing behaviours.

### Line ordering details

- Plain **Sort** compares whole line contents with **locale-aware collation** (Foundation `localizedCompare`, so e.g. `a < B < c`). It is not numeric. Numeric ordering ("Treat numbers as numeric values"), column/delimiter keys, regex-capture keys, ignore case, "Keep the first line at the top" and descending order are only in **Sort by Pattern…**. Those options default to ignore case on, language rules on and numeric on.
- Move Line Up or Down beeps and does nothing if any selection already touches the first line (or the last line, for Down).
- Duplicate Line puts the copy **below** and moves the selection(s) onto the copy. Pressing it repeatedly creates a downward column of copies.

### Multi-cursor clipboard rules

- **Copy** with multiple selections joins the pieces with the document line ending. It also remembers privately how many lines each piece had.
- **Paste** with N carets:
  - A single-line clipboard is inserted at every caret.
  - A clipboard that came from an N-way copy is split back so that each caret gets its own piece.
  - If there are more carets than pieces, the extra carets get empty strings.
- **Paste** normalizes line endings to the document's line ending. **Paste Exactly** does not.
- **Copy as Rich Text** copies the highlighted colours plus the font.

### Undo naming (shown in Edit → Undo/Redo titles)

Sort Lines · Reverse Lines · Shuffle Lines · Delete Duplicate Lines · Duplicate Line · Delete Line · Join Lines · Move Line · Trim Trailing Whitespace · Comment Out · Uncomment · Shift Right · Shift Left · Convert Indentation · Insert Snippet · Insert Color Code · Line Endings to %@ · Encoding to “%@” · (normalization form name, e.g. "NFC").

### Validation

- All editing commands are disabled when the document is read-only.
- Comment items are disabled when the syntax has no delimiters of the needed kind.
- Copy as Rich Text is disabled when nothing is selected.
- Close Split Editor is disabled with a single editor.
- Show Invisibles is disabled when all invisible types are switched off in Settings.
- Sidebar items are disabled outside folder windows.


## Find & Replace Panel

Source basis: `Sources/Text Finder/**`, `Packages/EditorCore/Sources/TextFind`, `Localizables/Text Finder/TextFind.xcstrings`. All labels below are the exact English strings.

### Related menu commands (Find menu, for context)

| Item | Shortcut | Action |
|---|---|---|
| Find… | ⌘F | Show the Find & Replace panel and focus the Find field (its whole text gets selected). Does not copy the selection. |
| Find Next | ⌘G | Next match from the end of the current selection. |
| Find Previous | ⇧⌘G | Previous match from the start of the current selection. |
| Find All | ⇧⌘F | Find All + show the result list in the panel. |
| Select All Find Matches | (none) | Select every match as a multiple selection. |
| — | | |
| Use Selection for Find | ⌘E | Find string := first selected range's text; also **turns off Regular Expression**. |
| Use Selection for Replace | ⇧⌘E | Replacement string := first selected range's text. |
| Jump to Selection | ⌘J | Scroll so the selection is centred. |
| — | | |
| Replace | (none) | Replace the current selection if it matches; does not move on. Beeps if the selection is not a match. |
| Replace and Find Next | ⌥⌘G | Replace the current selection (if it matches), then find next. |
| Replace All | (none) | Replace every match. |
| Multiple Replace ▸ | | Submenu: one item per saved Multiple Replace definition (runs it as Replace All on the frontmost document), separator, **Manage Multiple Replace…** (opens the Multiple Replace window). |
| — | | |
| Highlight All | ⇧⌘H | Highlight all matches without listing them. |
| Remove Highlights | ⌃⇧⌘H | Remove find highlights from the frontmost editor. |

Enablement: Find Next/Previous/Select All/Find All/Use Selection: need a selectable text view. Replace/Replace All/Multiple Replace: need an editable text view. Highlight/Remove Highlights/Show panel/Manage Multiple Replace: always enabled.

### Window

- One app-wide panel (singleton), title **"Find & Replace"**. It is a floating utility panel: always above document windows, closable, resizable, zoom button disabled, transparent title bar. It is not tied to one document; every action goes to the text view of the current main document window (the panel itself never becomes the target).
- Frame position and size are remembered ("Find Panel" autosave). While the panel is hidden, its saved height is not restored (only width), so it opens at the compact height.
- Layout is a vertical split view with three parts, top to bottom:
  1. **Field area** (Find field, Replace field, option row).
  2. **Find All result area**: collapsed by default; the divider is completely invisible and not draggable while it is collapsed.
  3. **Button row**.
- When the panel opens it focuses the Find field and selects all of its text. On panel close, the result area collapses again.

### Field area

Two multi-line text fields stacked vertically, each with a rounded border (radius 7 px, separator colour) and a minimum height of 44 px:

1. **Find** field: placeholder "Find", tooltip "Type the text to search for."
2. **Replace** field: placeholder "Replace with", tooltip "Type the text to replace the found text."

Field behaviour (both):
- They are real multi-line text views that behave like a field editor. Lines do **not** wrap: the field scrolls horizontally and vertically (small scrollers) and can grow or scroll with content. Tab and Shift-Tab move focus. Return runs the action (see below). An explicit newline can be typed (on macOS with ⌥Return).
- System font. Automatic quote, dash and text substitution, spelling correction and smart insert are all off. Undo is supported.
- Invisible characters are drawn in the fields with the same visibility settings as the editor (the global "show invisibles" setting and the per-type toggles), in a disabled-text colour. Control characters are drawn as replacement glyphs.
- Selecting all text on focus and clearing the selection on blur match standard single-line entry behaviour.
- **Return in the Find field = Find Next**; **Shift+Return = Find Previous**. Return in the Replace field does nothing.
- Each field shows **overlays in its top edge**:
  - **Leading: a history menu button**. The icon is a magnifying glass for Find and a pencil for Replace. The menu has a section header "Recent Searches" / "Recent Replacements" listing up to **20** recent strings, newest last as stored. Long entries are cut to 64 characters + "…", and the tooltip shows the full string. Choosing an entry sets the field. After the list comes **"Clear Recent Searches"** / **"Clear Recent Replacements"** (trash icon), disabled when the history is empty.
  - **Trailing: shown only when the field is not empty.** It holds a small grey result label and a circled-x **Clear** button that empties the field. The field gets extra trailing scroll inset so that text is not hidden under this overlay.
- Result label in the **Find** field:
  - After Find Next/Previous: **"current/total"**, for example `3/12`, with monospaced digits.
  - After Find All, Highlight All, Select All, or if the current position is unknown: **"N matches"** / **"1 match"** / **"Not found"**.
- Result label in the **Replace** field after Replace All: **"N replaced"** or **"Not replaced"**.
- The result label is cleared when the find string changes, when the replacement string changes (for a replace result), and when the document window stops being main. Only the position part is dropped (falling back to "N matches") when the user moves the selection away from the matched range.
- **Regex syntax colouring** applies when "Regular Expression" is on. Without it, the text is plain.
  - Find field: characters such as `.`, `\w`, `\d`, escapes and `[...]` contents are coloured; back references (`$1`, `\1`, `\k<name>`) are coloured; symbols (`(`, `)`, `|`, `(?:`, `(?=`, `\Q…\E`, brackets) are coloured; quantifiers (`* + ? {n,m}`) are coloured; anchors (`^ $ \b \A \z`…) are coloured. Text inside `\Q…\E` is not coloured. Escaped characters are respected.
  - Replace field: only `$n` and, when "Unescape replacement text" is on, `\0 \t \n \r \" \' \\ \$` are coloured.
  - Colours (light / dark):

    | Type | Light | Dark |
    |---|---|---|
    | character | #34657D | #6B9FB2 |
    | back reference | #AF2A85 | #F26DC8 |
    | symbol | #AE3122 | #F2776D |
    | quantifier | #6C12A0 | #B04CD9 |
    | anchor | #6A8031 | #9EB26B |

  - If the pattern is invalid, colouring is simply removed (there is no red marking).
  - With regex on, double-clicking a `(` / `)` / `[` / `]` in the Find field selects the whole bracketed group. Moving the caret highlights the matching bracket.
- The find string is the **same value used by Folder Find**. On macOS it is also synced with the system-wide find pasteboard: it is written on every change and read back when the app becomes active. **[macOS-only]** Linux has no shared find pasteboard, so keep it app-global (optionally persisted).

### Option row (under the fields, small controls)

From left to right:
- Checkbox **"Regular Expression"** (tooltip "Select to search with regular expression."). Default **off**.
- Small round **help "?" button** (tooltip "Show quick reference for regular expression syntax."). It toggles a popover (arrow at the bottom) that can be detached into a window. The popover contains the regex quick reference described below.
- Checkbox **"Ignore Case"** (tooltip "Select to ignore character case on search."). Default **off**.
- Checkbox **"In Selection"** (tooltip "Select to search text only from selection."). Default **off**.
- Flexible space.
- Icon-only button **"⋯" in a circle** (accessibility label "Advanced options", tooltip "Show advanced options"). It opens the Advanced Find Options popover with the arrow on the trailing edge.

These three checkboxes and every advanced option are **global persistent preferences** (shared by all windows and kept across launches).

### Advanced Find Options popover

Heading "Advanced Find Options" (semibold, secondary colour), then all small checkboxes:

General section (no header):
- **"Wrap search around"**, default **ON**. Find Next/Previous continue from the other end of the document.
- **"Search incrementally"**, default **ON**.

Section **"Textual Search"** (applies only when Regular Expression is off):
- **"Match only whole words"**, default off. Tooltip "Restrict search results to whole words."
- **"Distinguish characters strictly"**, default off. Tooltip "Exact character-by-character equivalence." (literal comparison, no Unicode canonical equivalence)
- **"Ignore diacritical marks"**, default off. Tooltip "Search ignores diacritical marks (for example, ö = o)."
- **"Ignore width differences"**, default off. Tooltip "Search ignores width differences in character forms (for example, ａ = a)."

Section **"Regular Expression Search"** (applies only when Regular Expression is on):
- **"Dot matches line separators"**, default off. Tooltip "Allow . to match any character, including newline characters (singleline)."
- **"Anchors match lines"**, default **ON**. Tooltip "Allow ^ and $ to match the start and end of lines (multiline)."
- **"Use Unicode word boundaries"**, default off. Tooltip "Use Unicode TR#29 to specify word boundaries"
- **"Unescape replacement text"**, default **ON**. Tooltip "Unescape metacharacters with backslash in replacement text." When on, `\0 \t \n \r \" \' \\` in the replacement are turned into the real characters before `$n` template expansion.

Bottom right: a help "?" button that links to the find how-to help page.

"Ignore Case" applies in both modes. The regex engine is ICU (NSRegularExpression): replacement templates use `$0..$n`, and `\` escapes `$`. A GTK clone using PCRE2/GRegex must translate `$n` templates.

### Regex quick reference popover

Title "Basic Regular Expression Syntax". Two columns separated by a divider. Each entry shows the term in medium weight, then its description.
- **Characters**:
  - `.` any character
  - `\R` new line character
  - `\t` tab character
  - `\w` word character
  - `\s` whitespace character
  - `\S` non-whitespace character
  - `\d` decimal number
  - `[A-Z]` any character in range A to Z
  - `[^A-Z]` any character not in range A to Z
- **Anchors**:
  - `^` beginning of the line
  - `$` end of the line
  - `\b` word boundary
- **Quantifiers**:
  - `?` 1 or 0 times
  - `*` 0 or more times
  - `+` 1 or more times
  - `{n,m}` at least n but not more than m times
  - `{n,}` at least n times
  - `{,n}` at least 0 but not more than n times
  - `{n}` n times
- **Extended Groups**:
  - `(?=subexp)` look-ahead
  - `(?<=subexp)` look-behind
- **Back Reference**:
  - `$1` first match

Footer: "The syntax conforms to the ICU Regular Expressions specifications." ("ICU Regular Expressions" is a link to the ICU user guide), followed by a help button.

### Button row (bottom)

From left to right:
- **"Find All"**: a pull-down button with a primary action. A click runs Find All (tooltip "Find and list all matches."). Its menu has **"Highlight All"** (highlighter icon) and **"Select All"**.
- **"Replace All"** (tooltip "Replace all matches with the replacement text.").
- Flexible space.
- **"Replace"** (tooltip "Replace the current selection with the replacement text, then find the next match."). This button runs **Replace and Find Next**, not the menu's plain Replace.
- A two-segment control, 70 px wide, with **‹ "Find Previous"** and **› "Find Next"** (tooltips "Find previous match." / "Find next match.").

### Search semantics

- The find string's line endings are converted to the document's line ending before searching. Replacement text containing newlines is also converted to the document's line ending.
- **Find Next/Previous**:
  - Starts from the selection end (next) or start (previous). The current selection is skipped if it is exactly a match.
  - The found range is selected and scrolled into view, with a "find indicator" flash on macOS. In a clone, use a brief highlight flash.
  - If wrapping happened, a centred **HUD** briefly appears over the editor: a large looping-arrow symbol (vertically flipped for a backward search), 72 px, on a blurred rounded square (radius 14). It fades out after 0.5 s. A "Search wrapped." accessibility announcement is also posted.
  - If nothing is found: a HUD with "arrow down to line" (next) or "arrow up to line" (previous) appears, plus a beep.
  - Each search adds the find string to the history.
- **With "In Selection" on**: matches are limited to the selected ranges (multiple selections are supported). Find Next selects the *first* match inside the selection and Find Previous the *last*. Because this replaces the selection, a second Find Next stays inside the found match.
- **Incremental search**:
  - Runs when "Search incrementally" is on, "In Selection" is off, the find string is not empty, and (in regex mode) the pattern is valid.
  - Typing in the Find field triggers a search after a 0.2 s debounce. It searches forward *including* the current selection, so the match under the caret stays selected while typing.
  - All matches get a background highlight (the "unemphasized selection" colour). This highlight is removed when the document window loses key status.
  - There is no beep or HUD when nothing is found, and nothing is added to history.
- **Highlight All / Find All**:
  - While running, the document is made read-only and a **progress sheet** is attached to the document window. It contains a 220 px progress bar titled "Find All" (or "Highlight All"), a status text of "Searching in text…" or "N matches found.", and a circled-x cancel button.
  - Matches are highlighted with the accent colour at 40 % alpha. With regex capture groups, each group gets its own hue: the base colour is hue-rotated evenly by (groups + 1).
  - If there are no matches, it beeps.
  - Previous Find All highlights are cleared (including in another document if the last result belonged to it).
- **Select All**: all matches become the selection (multi-cursor or multiple selection).
- **Replace All**:
  - Same progress sheet, titled "Replace All", with the status text "N matches replaced.".
  - The whole operation is one undo step named "Replace All".
  - With In Selection on, the resulting selection covers each replaced selection range.
  - If nothing was replaced, it beeps. The replacement string is added to the replace history.
- **Errors**:
  - An invalid regex shows an alert sheet attached to the Find panel (the panel is shown first). Message "Invalid regular expression", informative text = the regex engine's reason, with a Help button.
  - "In Selection" with nothing selected shows the same kind of sheet: "The option “in selection” is selected, although nothing is selected." / "Select the search scope in the document or deselect the “in selection” option."
  - An empty find string just beeps.

### Find All result area

Appears automatically after a Find All with ≥1 match. It expands to at least 200 px tall, and the panel is brought to the front. Contents:
- Header row: a small **chevron-up "Close"** button (tooltip "Close find result.") that collapses the area. Next to it, bold text **"Found N matches in “<document name>.”"**, with plural forms: "Found one match in …", "Found no matches in …".
- Line: **"Find text: <find string>"**.
- Bordered table using small text. The size is adjustable and persisted; the default is the small system size:
  - Column **"Line"**: line number, right-aligned, monospaced digits, ideal width 30, max 64. Sortable; the default sort is by document position.
  - Column **"Matched Text"**: the line's text with the match (and capture groups) background-coloured exactly like the editor highlights. If needed, the line is truncated at the head so the match stays visible with about 16 characters of context before it, and tail-truncated at the end. At most 1024 characters around the match are taken from very long lines.
- Selecting exactly one row selects that match in the document and flashes it. This is ignored if the text has become shorter than the range.
- Copy (⌘C) copies the plain line strings of the selected rows.
- Context menu **"Text Size" ▸ "Bigger" / "Smaller" / "Reset to Default"**. The standard Bigger/Smaller/Reset font commands (⌘+ / ⌘− / ⌘0) also work while the table is focused. The minimum size is the small system font size.
- Collapsing the area (by the button, by dragging the divider to zero, or by closing the panel) **removes the Find All highlights** from the document.

---

## Multiple Replace

Sources: `Text Finder/Multiple Replace/**`, `Storyboards/Base.lproj/MultipleReplaceView.storyboard`, `MultipleReplace.xcstrings`, `MultipleReplaceView.xcstrings`.

### Window
- An app-wide floating utility panel titled **"Multiple Replace"**. It is resizable, its zoom button is disabled, and its frame is remembered. It opens from Find ▸ Multiple Replace ▸ **Manage Multiple Replace…**.
- It is a two-column split view:
  - Left: a **sidebar list of definitions** (min width 80, ideal 200).
  - Right: the **rule editor** for the selected definition.
- The selected definition name is persisted.
- If no definition exists, one named **"Untitled"** is created automatically (subsequent ones get unique numbers).

### Sidebar (definition list)
- Each row is the definition name and can be renamed in place.
- A row can be dragged out to export the file, and `.cotrpl` or `.tsv` files can be dropped in to import them.
- Bottom bar: **+** (tooltip "Add new item") creates "Untitled". **−** (tooltip "Delete selected items") asks "Are you sure you want to delete “NAME”?" / "This action cannot be undone." with a destructive "Delete" button.
- On the right of the bottom bar is a **⋯ Actions** pull-down:
  - Duplicate “NAME”
  - Rename “NAME”
  - Export “NAME”…
  - (Option-key alternate: Reveal “NAME” in Finder **[macOS-only]**)
  - Share **[macOS-only]**
  - divider
  - **Import…**
  - (Option-key alternate: **Reload All Definitions**)
- The row context menu has Duplicate, Rename, Delete, Export…, Reveal in Finder (alternate) and Share.
- Import dialog message: "Choose CotEditor Replace Definition or TSV (Tab-separated values) files.", confirm button "Import".
  - Name collision asks: "“NAME” already exists. Do you want to replace it?" with a "Replace" button.
  - TSV import: each line is `find<TAB>replace`. The find text must be non-empty; invalid lines are skipped.
- Storage: one JSON file per definition (`.cotrpl`) in the app's "Replacements" settings folder. Structure:
  - `replacements: [{findString, replacementString, usesRegularExpression, ignoresCase, isEnabled, description?}]`
  - `settings: {textualOptions, regexOptions, matchesFullWord, unescapesReplacementString}`

### Rule editor (right side)
- Top text: **"Perform replacement using these rules in succession from the top down:"**
- Table columns, in order:
  1. Checkbox (header tooltip "Enable"; cell tooltip "Select to use this replacement rule."), 20 px.
  2. **"Find"**, regex-highlighted when the rule uses regex. Shows a warning icon if the rule is invalid; the icon's tooltip is "[error] suggestion".
  3. **"Replace With"**, regex-highlighted for `$n`.
  4. **"RE"** checkbox (header tooltip "Regular Expression"), 28 px.
  5. **"IC"** checkbox (header tooltip "Ignore Cases"), 28 px.
  6. **"Description"**.
- Disabled rules show all their other controls disabled.
- Toggling a checkbox in a row that belongs to a multi-row selection applies the change to all selected rows.
- Rows can be reordered by drag and drop. Dragging a row to the Trash deletes it **[macOS-only gesture]**.
- Edits support undo, with the action names "Insert Rule", "Delete Rules", "Move Rules" and "Edit Rule".
- Changes auto-save 1 s after the last edit, and also when the window closes.
- Under the table, a **+ / −** segmented control (tooltips "Add new item" / "Delete selected items"):
  - **+** inserts an empty rule after the selection and starts editing its Find cell.
  - Deleting the last rule leaves one empty rule.
- Bottom bar:
  - Left: a help "?" button.
  - A warning icon + **"Invalid rules will be skipped."**, visible only after a run that found invalid rules.
  - A result text such as "5 replaced" / "Not replaced" / "N matches".
  - Right side:
    - Checkbox **"In Selection"** (tooltip "Select to replace text only in selection."). This is the **same global setting** as the Find panel's In Selection.
    - Action-gear button (tooltip "Show advanced options"). It opens a popover with the same "Advanced Find Options" Textual Search / Regular Expression Search sections, stored **per definition**: "Match only whole words", "Distinguish characters strictly", "Ignore diacritical marks", "Ignore width differences", "Dot matches line separators", "Anchors match lines", "Use Unicode word boundaries", "Unescape replacement text".
      - Per-definition defaults: all off except "Anchors match lines" ON and "Unescape replacement text" ON.
      - Wrap and Incremental are not shown here.
    - **"Highlight All"** (tooltip "Highlight all matches of the replacement rules.")
    - **"Replace All"** (tooltip "Replace all matches according to the replacement rules.").
- Semantics:
  - Enabled rules are applied **sequentially** from top to bottom. Each rule runs on the output of the previous one, and in In Selection mode the selection is carried along.
  - Invalid rules are silently skipped.
  - The whole run is a single undo step named "Replace All". A progress sheet appears on the document window, counting rules as progress units.
  - Highlight All highlights the union of all rule matches.
  - Running from the Find ▸ Multiple Replace submenu behaves the same as Replace All here.

---

## Find in Folder

(Sidebar pane of a **folder window**. Sidebar tab label "Folder Find", magnifying-glass icon. Menu: View ▸ Sidebar ▸ **Folder Find** ⌃⌘2. File Browser is ⌃⌘1.)

### Top controls (pinned above the list)
- Row 1, small controls:
  - A **pull-down picker** (no visible label; accessibility "Search method") with "Text" or "Regular Expression". The current value is shown as plain text: "Text" in normal colour, "Regular Expression" in the accent colour. Default Text.
  - Flexible space.
  - On the right, an icon toggle **"Case Sensitive"** (icon "textformat"; bold when on). Default **off**, meaning ignore case.
- Row 2: a **search field**, placeholder **"Search in Folder"**, with the native recent-searches menu (autosaved as "FolderSearch").
  - Its text is **the same find string as the Find panel**.
  - In regex mode the field is regex-highlighted.
  - **The search starts only on Return.** Editing the text cancels a running search and resets the "searching" state.
- Row 3: the scope menu button **"Scope"**, magnifying-glass icon. It shows the saved scope's name when one is selected, and turns accent-coloured when a scope is active. Its menu:
  - **"Include Hidden Files"** (toggle, default off)
  - **"Include Other File Types"** (toggle, default off)
  - **"Edit File Scope…"**
  - **"Clear File Scope"** (disabled if there is no scope)
  - If saved scopes exist: a **"Saved Scopes"** inline radio list of names, then **"Manage Saved Scopes…"**.
- Under the controls, while searching and after a search: a divider, a secondary-coloured small summary **"N matches in M files"**, or **"N+ matches in M files"** when truncated, then a divider. It updates every 0.1 s during the search.

### Result list
- Files appear as expandable groups, expanded by default. Each group label has:
  - a doc icon;
  - the **file name** in medium weight;
  - the relative directory path in secondary colour (tooltip = full path).
- Under each file, one row per match: an icon, then the line text in secondary colour with the matched part **bold and in primary colour**. Leading indentation is trimmed. The line is head-truncated so that about 32 characters of context precede the match, and it may wrap by character, up to 3 lines.
- If the per-file cap is reached, the file ends with: "Only the first N matches were found."
- If the global cap is reached, the list ends with a centred "Search Limit Reached".
- Overlays:
  - while searching: a spinner with "Searching in folder…";
  - with zero matches: an empty state "No Results" with "No matches for “X” were found.";
  - on error: "Search Failed" with the error text (for example "The folder cannot be found.", or the regex error).
- Selecting one row opens the file in the folder window's editor (a file row opens it, a match row opens it and selects that match). All matches of that file are highlighted with the unemphasized selection colour, the selected match is flashed, and the highlight is removed when the editor gains focus.
- Delete (⌫) removes the selected result rows from the list.
- File rows can be dragged out as files.
- Context menu (single selection):
  - "Reveal in File Browser"
  - "Show in Finder" **[macOS-only; use "Show in Files"/file manager]**
  - "Open in New Window"
  - divider
  - "Open with External Editor"
- Results stay in sync with edits: match ranges shift as open documents are edited.

### Search rules
- Recursive. Folders come first, then files, in natural (Finder-style) name order.
- Hidden files are skipped unless included. `.DS_Store` and `.git` are always excluded. Symlinks and aliases are never followed.
- A file is searched if:
  - it is a "text" type, a property list, or has no extension; or
  - its name matches a syntax's file mapping; or
  - "Include Other File Types" is on.

  It is still skipped if it is binary (sniffed) or bigger than 1 GB.
- Files are decoded using the user's encoding-detection candidate list.
- Caps: **500,000 matches in total** (after which the remaining files are not searched) and **5,000 matches per file**. Line excerpts are limited to about 512 UTF-16 units around the match.
- A file scope applies on top of these rules.

### Edit File Scope sheet
- If editing a named scope, the first row is a **"Name"** field.
- Sentence row: **"Match [any|all] of the following conditions:"**, with a popup holding "any" (default) and "all".
- A rule editor (like macOS's NSRuleEditor; use a list of rows in GTK). Each rule is [**target**] [**comparison**] [value field] with +/− buttons:
  - Targets: **"File name"**, **"File path"** (relative to the folder root), **"File extension"**.
  - Comparisons: **"contains"**, **"does not contain"**, **"is"**, **"is not"**, **"begins with"**, **"ends with"**, **"matches regular expression"**.
  - Non-regex comparisons are case-insensitive. A regex must match the whole value and is case-sensitive unless `(?i)` is used. In regex mode the value field is regex-highlighted.
- Validation message below the editor (reserved height): "The scope contains a rule without a value." or "The scope contains an invalid regular expression."
- Bottom: help "?" link, then (unnamed scope only) **"Save as Named Scope…"** (disabled while the scope is empty), then space, then **Cancel** and **OK**.
- **"Save as Named Scope…"** opens a small sheet with **"Save as:"** [Name] and Cancel / **"Save"**. Name errors include "Name can’t be empty." and "The name “X” is already taken.".
- Min width 400, ideal 540.

### Manage Saved Scopes sheet
- "Saved scopes:" above a bordered list of names.
- Bottom bar: + ("Add new item"), − ("Delete selected items"), pencil **"Edit…"** ("Edit selected item").
- Double-click edits a scope. The context menu has Duplicate and Delete.
- A **Close** button (default action) closes the sheet.
- Scopes are stored in preferences.

---

## Go To Line

- Menu: Find ▸ **"Go to Line…"** ⌘L. Opens a **sheet on the document window**.
- Contents:
  - One text field with placeholder **"Line Number"** and monospaced digits.
  - Under it, the standard button row: help "?" (left), **Cancel** (Esc), **"Go"** (Return, default).
- Input syntax ("fuzzy range"): `N` or `N:M`.
  - `N` is the 1-based line. A negative N counts from the end (`-1` = last line). `0` puts the caret at the very start.
  - N greater than the line count puts the caret at the very end.
  - `M` is the number of lines to select (M=0 or 1 selects one line). A negative M means "up to the Nth line from the end" (`3:-1` = line 3 through the last line).
- Effect: selects the whole line(s) **including the trailing line ending** and scrolls to them.
- If the input is invalid or out of range, it beeps and the sheet stays open. On success the sheet closes.

---

## Character Inspector

- How it is opened:
  - Menu Text ▸ **"Inspect Character"** ⌥⌘I. It is enabled only when exactly one character (one grapheme cluster) is selected.
  - The editor's context menu gets an **"Inspect Character"** item inserted at position 1 under the same condition.
- Opening it scrolls to the character and flashes it. A **popover** then appears below the character, anchored to its rect inset by 4 px.
  - The popover is "semi-transient": it closes when you click in the editor, but not when you click in other windows.
  - It can be **detached** by dragging it into a free-floating window.
- Layout (14 px padding), horizontal:
  - **Left: the glyph** at 64 pt in a serif font, min width 64. Control characters and whitespace show their Unicode "control picture" (for example ␊ for LF, ␉ for tab) in tertiary colour.
  - **Right: details.**
    - First line: the Unicode name (semibold), for example "LATIN SMALL LETTER A".
      - For variation sequences, the variant is appended in parentheses, for example "HEAVY CHECK MARK (Text Style)".
      - For multi-scalar clusters: "<a letter consisting of N characters>" in regular weight.
      - With no name: "Unknown" in secondary colour.
    - For single-scalar characters, a small two-column grid (labels right-aligned):
      - **"Code Point:"** `U+XXXX`; for non-BMP characters the UTF-16 surrogate pair follows, for example `U+1F600 (U+D83D U+DE00)`. If the code point is deprecated, a bordered **"deprecated"** badge is shown on the right.
      - **"Block:"** the localized block name. Clicking toggles between the localized and English name. "No Block" is shown if there is none.
      - **"Category:"** `Long Name (Lt)`, for example "Lowercase Letter (Ll)".
      - **"Version:"** `Unicode X.Y` (the age).
    - For clusters of several scalars: a list of **disclosure rows**, one per scalar. The label is the code point (min width 44, monospaced digits) plus the name in medium weight. Expanding a row shows that scalar's glyph (28 pt serif, bordered box) next to Block and Category.
- All text is selectable.

---

## Unicode Input and Other Small Sheets

### Insert by Unicode Code Point…
- Menu Edit ▸ **"Insert by Unicode Code Point…"**. No default shortcut. Enabled when the editor is editable.
- A **popover under the caret** (transient), 160 px wide:
  - Row: a 26 pt preview of the character (or "⬚" when invalid), then a small 2-line text with the Unicode name, falling back to the code point. When the input is invalid it shows "Invalid code" in secondary colour.
  - Text field with placeholder **`U+1F600`** and a leading 18 px inset that holds a borderless **history menu**:
    - Section "Recents": up to 10 entries, each shown as the code point padded to 9 characters in a monospaced font, followed by the name in small secondary text.
    - "Clear Recents" (destructive), shown only when there are recents.
- Accepted input: `U+hhhh`, `0xhhhh`, `\uhhhh` or bare hex, 1–6 hex digits, case-insensitive.
- Return inserts the character at the selection, keeping the field open and clearing it, and adds the code point to the recents. Invalid input beeps.

### Custom Tab Width sheet
- Opened from Format ▸ Tab Width ▸ "Custom…".
- **"Tab width:"** followed by a number field with a stepper. Range 1–99, prefilled with the current width.
- Buttons: Cancel and **OK**. Return also submits.

### Custom Surround sheet
- Opened from Text ▸ Surround Selection With ▸ "Custom…" (⇧⌘L). The other items in that submenu directly surround with `' '`, `" "`, `( )`, `{ }` and `[ ]`.
- Bold title **"Surround with:"**.
- Row: **"Begin:"** [48 px field] and **"End:"** [48 px field].
  - The End field's placeholder mirrors the Begin text.
  - If End is left empty, the Begin string is used for both sides.
- Buttons: Cancel and **OK**. An empty Begin beeps.
- The last pair used is remembered and prefilled next time, both per editor and globally.
- Works for every selection (multi-cursor aware).

### Sort by Pattern… sheet
- Opened from Text ▸ Lines ▸ "Sort by Pattern…". Needs an editable document. Min width 460.
- **"Sample:"** (section) shows a group box with one line from the document: the line at the start of the selection, or the first line of the document. It uses the editor font, is truncated, and has the tooltip "Sample line to check which part of a line will be used for sorting.". The part used as the sort key is highlighted live with the accent colour at 30 %.
- **"Sort key:"**, a horizontal radio group:
  - **"Entire line"**
  - **"Column"** (default)
  - **"Regular expression"**
- Extra controls depending on the sort key:
  - Column: **"Delimiter:"** [32 px field, placeholder `,`; escapes like `\t` are allowed] and **"Position:"** [stepper field, 1-based, default 1]. The column value is whitespace-trimmed before sorting.
  - Regular expression:
    - **"Pattern:"** regex field (placeholder "Regular Expression", regex-highlighted) with a leading history menu: "Recents" (last 10) and "Clear Recents".
    - Under it: a small "Ignore case" checkbox (default ON). An inline error label can appear: "Empty pattern" or "Invalid pattern".
    - **"Use captured group:"** checkbox (default off) with a stepper number field (default 1, range 0…number of groups). The field is disabled unless the checkbox is checked.
- **"Sort option:"** checkboxes:
  - "Ignore case" (default **ON**)
  - "Respect language rules" (default **ON**, locale-aware comparison)
  - "Treat numbers as numeric values" (default **ON**)
  - "Keep the first line at the top" (default off)
  - "In descending order" (default off)
- Buttons: help "?", Cancel, **"Sort"**. "Sort" is disabled while the pattern is invalid.

### Advanced Character Count
- Menu Text ▸ **"Advanced Character Count…"**. While the counter is shown, the item's title changes to **"Stop Advanced Character Count"**.
- It first opens an **options sheet** containing the options view below, then help, Cancel and **"Start"**.
- Options view, a two-column grid:
  - **"Whitespace:"**
    - "Ignore line endings" (default off)
    - "Ignore whitespace" (default off)
    - "Treat consecutive whitespace as one space" (default off; disabled when both ignores are on)
  - **"Unit:"** popup:
    - **"Grapheme cluster"** (default), with the description text "Count in the intuitive way defined in Unicode. A character consisting of multiple Unicode code points, such as emojis, is counted as one character."
    - "Unicode scalar", "Count Unicode code points. Same as counting UTF-32."
    - "UTF-16", "Count Unicode code points but a surrogate pair as two characters."
    - "Byte", "Count bytes of the text encoded with the specified encoding."
    - The description of the selected unit is shown below the popup in small secondary text.
    - When Byte is chosen, an **"Encoding:"** popup lists all encodings, default UTF-8.
    - When the unit is not grapheme cluster: a **"Normalization:"** checkbox (default off) plus a form popup (NFC default; standard forms and modified forms in two sections).
  - All options are global preferences.
- After Start, a **floating capsule** ("glass" background, about 12×8 px padding) appears over the editor, in the top-right corner for horizontal LTR text. It can be dragged anywhere inside the editor.
  - It shows the count of the **selection if there is one, otherwise the whole document**, live-updated: "**123** characters" (bold, monospaced number; "character" when the count is 1), or a binary byte count in Byte mode.
  - On failure it shows a warning "failed".
  - A gear toggle ("Show options") opens the same options view in a popover.
  - Context menu: "Copy" (copies the number) and "Stop Count".
  - Its visibility is restored with window state.

### Edit Color Code… (color panel)
- Menu Text ▸ **"Edit Color Code…"** ⌃⌘H. It opens the system colour panel prefilled from the selected text if that text parses as a colour code. **[macOS-only colour panel; in GTK use a GtkColorChooser dialog/popover with the same accessory.]**
- The panel shows alpha, and adds a colour list named "Stylesheet Keywords" (CSS named colours).
- Accessory under the panel:
  - A centred monospaced 14 pt text field, placeholder **"Color Code"**. Return applies a typed code to the panel.
  - Row: a type popup and an **"Insert"** button (the default button).
    - Popup section 1: "Hexadecimal" (default), "Hexadecimal with Alpha", "Hexadecimal (Short)", "Hexadecimal with Alpha (Short)".
    - Popup section 2: "CSS RGB", "CSS RGBa", "CSS HSL", "CSS HSLa", "CSS HWB", "CSS HWB with Alpha", "CSS Keyword".
- The code updates live as the colour changes.
  - CSS Keyword falls back to Hexadecimal when the colour has no keyword name.
  - Hex output uses uppercase if the user typed uppercase.
- "Insert" replaces the selection in the frontmost editor with the code and selects it. It is one undo step, "Insert Color Code". It beeps if the code is invalid.

### Insert from iPhone/iPad / Live Text image insertion **[macOS-only]**
- Continuity Camera / Live Text OCR insertion. The messages are "Detection failed" and "No text detected". Skip it in a Linux clone.

### About / What's New (one line each)
- **About**: window "About CotEditor" with the app icon, "Version X (build)", and the panes "Credits" and "Licenses" (Code Contributors, Localization, Special Thanks, "The CotEditor Project", open-source licence list).
- **What's New**: a sheet-like window "What’s New in CotEditor X" listing new features, with "Release Notes" and "Continue" buttons.

---

## Snippets Behaviour

- Snippets are defined in Settings ▸ Snippets (see the Settings section). Each one has a **name**, an optional **scope** (a syntax name, for example "Markdown"; empty = all), an optional **shortcut**, and a **format** text.
- Menu: Text ▸ **"Insert Snippet" ▸**
  - First, all general snippets (no scope) as items, then a separator.
  - Then, if the current document's syntax has snippets, a **section header with the syntax name** followed by those snippets, then a separator.
  - Last, **"Edit Snippets…"**, which opens the Settings Snippets pane.
- The menu rebuilds whenever the frontmost document's syntax changes. Snippets of other scopes are hidden.
- Shortcuts work as menu key equivalents. A general snippet loses its shortcut while a scoped snippet with the same shortcut is visible, so the scoped one wins.
- Variables in the format:
  - **`<<<SELECTION>>>`**: the selected text ("The selected text."). It is replaced per selection.
  - **`<<<CURSOR>>>`**: where the insertion point goes after insertion ("The insertion point after inserting the snippet."). **Several `<<<CURSOR>>>` tokens produce several insertion points** (multi-cursor). With none, the caret goes after the inserted text.
- Insertion semantics:
  - Works on **every selection/caret** (multi-cursor): each range is replaced by the expanded snippet.
  - Each newline in the format is followed by **the indentation of the line where that selection starts**, so multi-line snippets keep the indent.
  - The text is split at `<<<CURSOR>>>` *before* the selection is substituted, so a literal `<<<CURSOR>>>` inside the selected text is kept as-is.
  - One undo step, "Insert Snippet". It scrolls to the selection afterwards.
- **File drop snippets** (Settings ▸ Snippets, "File Drop" part) control what is inserted when files are dropped on the editor.
  - Each entry has a format, an optional extension list and an optional scope.
  - The first entry whose scope matches the document syntax (or has no scope) and whose extension list contains the file's extension (case-insensitive; an empty list matches any) is used.
  - Tokens:

    | Token | Value |
    |---|---|
    | `<<<ABSOLUTE-PATH>>>` | absolute path |
    | `<<<RELATIVE-PATH>>>` | path relative to the document (absolute if the document is unsaved) |
    | `<<<FILENAME>>>` | file name |
    | `<<<FILENAME-NOSUFFIX>>>` | file name without extension |
    | `<<<FILEEXTENSION>>>` | extension |
    | `<<<FILEEXTENSION-LOWER>>>` | extension, lowercase |
    | `<<<FILEEXTENSION-UPPER>>>` | extension, uppercase |
    | `<<<DIRECTORY>>>` | parent folder name |
    | `<<<FILECONTENT>>>` | file content, read as UTF-8 |
    | `<<<IMAGEWIDTH>>>` | pixel width |
    | `<<<IMAGEHEIGHT>>>` | pixel height |

  - If no entry matches, the absolute path is inserted. Several dropped files are joined by line endings.
  - `.textClipping` files insert their text **[macOS-only]**.
  - Defaults:
    - Markdown, for jpg/jpeg/gif/png: `![<<<FILENAME-NOSUFFIX>>>](<<<RELATIVE-PATH>>>)`
    - Markdown, any file: `[<<<FILENAME-NOSUFFIX>>>](<<<RELATIVE-PATH>>>)`
    - HTML, for images: `<img src="<<<RELATIVE-PATH>>>" alt="<<<FILENAME-NOSUFFIX>>>" title="<<<FILENAME-NOSUFFIX>>>" width="<<<IMAGEWIDTH>>>" height="<<<IMAGEHEIGHT>>>" />`
    - HTML, for js: `<script type="text/javascript" src="<<<RELATIVE-PATH>>>"></script>`
    - HTML, for html/htm/php: `<a href="<<<RELATIVE-PATH>>>" title="<<<FILENAME-NOSUFFIX>>>"></a>`
    - CSS: `url("<<<RELATIVE-PATH>>>")`

---

## Scripts Menu

### Location and discovery
- The scripts folder is **`~/Library/Application Scripts/com.coteditor.CotEditor/`**, the sandbox "Application Scripts" directory. **[macOS-only path.]**
  - **Proposed Linux equivalent:** `$XDG_DATA_HOME/<app-id>/scripts/` (default `~/.local/share/<app-id>/scripts/`). Create it on first launch. Watch it with a GFileMonitor (recursive), which plays the role of CotEditor's file presenter.
- The menu is a top-level **"Script" menu** shown as an **icon** in the menu bar (the AppleScript scroll glyph). It is rebuilt:
  - at launch;
  - whenever the folder content changes: debounced 0.2 s, and if the app is in the background, deferred until it becomes active.
- Building rules, applied recursively per folder:
  - Hidden files (dot-files) are skipped. **Names starting with `_` are ignored** (files and folders).
  - Entries are sorted by full file name, in natural / Finder-like order.
  - **Ordering prefix:** a leading `digits)` is stripped from the displayed name. For example `01)Format JSON.py` is shown as "Format JSON". Use it to control order.
  - **Separator:** an entry whose name (after stripping the extension and prefix) is exactly `-` becomes a menu separator. For example a file `05)-` or a folder named `-`.
  - **Sub-folders** become submenus with the folder name (prefix stripped). Empty folders still appear.
  - **Script files** become items. The recognised types (by content type / extension):
    - Unix scripts: shell (`.sh`, `.command`, `.zsh`…), Perl (`.pl`), PHP (`.php`), Ruby (`.rb`), Python (`.py`), JavaScript (`.js`), AWK (`.awk`), Swift (`.swift`).
    - AppleScript: `.applescript`, `.scpt`, `.scptd` (bundle) **[macOS-only]**.
    - Other files are ignored. A bundle that is a script is treated as a script, not as a folder.
  - **Keyboard shortcut in the file name:** a second extension right before the real one encodes the shortcut in "key spec" notation. The shortcut part is removed from the displayed name.
    - Example: `Sort Lines.@~s.py` → item "Sort Lines" with ⌘⌥S.
    - Modifier characters: `@` = Command, `~` = Option, `$` = Shift, `^` = Control, `#` = numeric pad. Any number of modifiers, then the key.
    - An uppercase letter key is used as typed (e.g. `@$s` or `@S`).
    - The shortcut must be "assignable": it needs at least one modifier from Control/Option/Shift/Command, except that navigation and function keys (Home, End, PgUp, PgDn, F1…) may stand alone. Otherwise it is ignored and the name is kept as-is.
    - Linux mapping: Command → Ctrl (or Super), Option → Alt.
  - **Shortcut priority by syntax:** if a top-level sub-folder is named exactly like the **current document's syntax** (for example a folder "Markdown"), its scripts' shortcuts are applied first. The remaining scripts get theirs only when there is no conflict. Shortcuts are re-applied every time the frontmost syntax changes. (No check against built-in app shortcuts is made; the clone should decide how to handle such clashes.)
  - At the end of the menu: separator, then **"Open Scripts Folder"** (folder icon), which reveals the folder in the file manager.
  - Every script item has the tooltip **"Option-click to open script in editor."**
- Modifier-clicking a script item:
  - **⌥-click** opens the script file itself (in CotEditor / the default app).
  - **⌥⇧-click** reveals it in Finder.
  - A plain click runs it.
  - (Linux: map ⌥ to Alt; reveal goes through `org.freedesktop.FileManager1`.)
- Help ▸ "CotEditor Scripting Guide" ▸ "About Scripting", "AppleScript Dictionary" **[macOS-only dictionary]**.

### Running a Unix script
1. Checks, each showing an alert on failure:
   - The file must exist: "The script “X” does not exist."
   - It must be **executable** (+x): "The script “X” can’t be executed because you don’t have permission to execute it." / "Check the permission of the script file."
   - It must be readable as non-empty UTF-8: "The script “X” couldn’t be read." / "Check the script file."
2. The script **source text is scanned** for header tokens anywhere in the file (typically in a comment), using the regex `%%%{TOKEN=Value}%%%`, where Value is `\w+`. The first occurrence wins. Values are **case-sensitive**, and an unknown value means "not specified".
3. The script is executed directly (the shebang decides the interpreter), with **argument `$1` = the frontmost document's file path** if it has been saved. Otherwise there are no arguments.
4. **stdin** = the input text (UTF-8) if `CotEditorXInput` is given; otherwise stdin is empty (`/dev/null`).
5. **stdout** is captured only if `CotEditorXOutput` is given. **stderr** is always captured.
6. After the process exits, stdout and stderr are both read in full. The output is applied *first*. Then, **if stderr is not empty, it is reported as an error** in the Console, even if the exit status was 0.

`%%%{CotEditorXInput=…}%%%` values ("current document" = the frontmost plain-text document window):

| Value | stdin content |
|---|---|
| `Selection` | Text of the (first) selected range. |
| `CurrentLine` | The full line(s) touching the selection, **including the trailing line ending**. |
| `AllText` | Whole document text. |
| (absent) | Nothing. |

If an input is requested but there is no document, the error is "No document to get input from."

`%%%{CotEditorXOutput=…}%%%` values:

| Value | Effect with stdout |
|---|---|
| `ReplaceSelection` | Replace the (first) selection with the output, then select the inserted text. |
| `ReplaceCurrentLine` | Replace the full line(s) touching the selection (including the line ending) and select the output. |
| `ReplaceAllText` | Replace the whole document and select all of it. |
| `InsertAfterSelection` | Insert the output as an insertion at the selection and select it. **Note:** the current code inserts at the selection's *start* location (zero-length range at `selection.location`), despite the name. Documented intent is "after the selection"; a clone should decide deliberately. |
| `AppendToAllText` | Append the output at the end of the document and select it. |
| `NewDocument` | Open a new untitled document containing the output, only if the output is not empty. |
| `Pasteboard` | Put the output on the clipboard. |
| (absent) | Output is discarded (not captured). |

- The document-modifying outputs are one undoable edit. They fail with "No document to put output into." if there is no document, or "The document is not editable." if it is read-only.
- An empty stdout with a Replace* type replaces the target with an empty string.
- Output is decoded as UTF-8.

### Errors
- **Script runtime errors** (stderr text, no input/output target, not editable) are appended to the **Console**, titled with the script name. **The Console window is then shown.**
- File-level errors (missing, permission, unreadable, cannot open) are shown as a modal alert.

### AppleScript / JXA scripts **[macOS-only]**
- `.applescript`, `.scpt` and `.scptd` run through OSA.
- A `.scptd` bundle may declare these keys in its `Contents/Info.plist`:
  - `CotEditorExecutionModel` = `unrestricted` (default) or `persistent` (keeps one script instance alive between runs);
  - `CotEditorHandlers` = an array containing `document opened` and/or `document saved`. The script's handler is then called automatically, with the document as parameter, whenever any document is opened or saved.
- The scripting dictionary also provides a `write to console` command (with `title` and `timestamp` booleans, both default true).
- **No direct Linux equivalent.** Optionally offer "hook" scripts (for example a `hooks/document-opened/` folder) that receive the path as an argument.

---

## Console

- Window ▸ **"Console"** (terminal icon). No default shortcut. It also opens automatically when a script reports an error or calls `write to console`.
- A regular (non-floating) utility panel titled **"Console"**. Default size 360×200, resizable, frame remembered. It stays visible when the app is inactive.
- Compact unified toolbar with one icon-only bordered button, **"Clear Log"** (trash icon, tooltip "Clear Log"), which empties the log.
- Body: a read-only, selectable, scrollable plain text view with a 4 px vertical inset. Incremental find is supported (⌘F inside it).
- Each entry:
  - Header line: `[YYYY-MM-DD HH:MM:SS]` (local time, ISO-8601 style, monospaced digits), a space, then the **script name in semibold**. The timestamp and/or title can be omitted per entry, and the header line is skipped if both are absent.
  - Then the message in a **monospaced** font, indented 10 px (first line and continuation lines), followed by a newline.
- The view auto-scrolls to the newest entry. Entries are not persisted between launches.
- Text size: the Bigger / Smaller / Reset font commands work while the Console is focused. The minimum (and default) is the small system font size, and the setting is persisted.

---

## Quick Actions (Command Bar)

- View ▸ **"Show Quick Actions…"** ⌘K. Pressing it again while the bar is open closes it.
- A borderless floating panel, 500 px wide, centred on screen. It has a translucent "glass" background, no title-bar buttons, and can be moved by dragging its background. It **closes on Esc or when it loses focus**.
- Top row: a magnifying-glass icon and a large (20 pt, light weight) plain text field with placeholder **"Quick Actions"**. The input is cleared each time the bar opens.
- When candidates exist, a divider and then a scrollable list (max height 300, 6 px row spacing, 10 px vertical margins). Each row has:
  - Left: a 26×22 icon: a menu glyph for **"Command"** items, an AppleScript glyph for **"Script"** items.
  - Middle line 1: the matched path components plus the title, joined by small "›" chevrons, with the matched characters in **bold**.
  - Middle line 2: the full menu path in small secondary text (for example "Format › Syntax").
  - Right: the item's shortcut, shown as keycaps.
  - The selected row has an accent-colour rounded background (radius 7) with selected-menu-text colours.
- Candidate source: **every enabled, visible, leaf item of the main menu** at the moment the bar opens. Dynamic submenus are refreshed first. "Show Quick Actions…" itself is excluded.
- Matching:
  - Abbreviation (subsequence) matching, case-insensitive, done in order over the item's submenu path components (excluding the top-level menu name) and then its title.
  - Whitespace after a matched segment separates components, so "syn sw" finds Format › Syntax › Swift.
  - All typed characters must be consumed.
  - **Score** = the span length from the first to the last matched character in each matched component, summed. **Lower is better**; results are sorted by ascending score.
  - The first candidate is pre-selected. An accessibility announcement says "N commands found" / "No commands found".
- Keys: ↑/↓ move the selection (scrolling it into view). Return performs it. Clicking a row (without dragging) performs it.
- Performing: the bar closes **first**, so that the action goes to the document window that was in front, and then the menu action runs exactly as if the menu item had been chosen (same tag and represented object).


## Settings Window

Source: `CotEditor/Sources/Settings Window/**`, `Packages/MacUI/Sources/ControlUI/SettingsWindow/*`, defaults in `CotEditor/Sources/Application/Defaults/DefaultSettings.swift`, labels from `CotEditor/Localizables/Settings/**`.

### Window style and behaviour

- Classic macOS preferences window: a **toolbar with icon+label tab buttons**, one pane visible at a time. Window is titled and closable only (not resizable, no minimize/zoom). Window title = the label of the selected pane (e.g. "General").
- Content width is fixed at **580 pt** for every pane; height is the pane's natural height. When switching panes the window **animates its height** (top edge stays fixed, window grows/shrinks downward); the new pane's view is hidden during the resize animation and the title updates when the animation ends. No animation when "Reduce motion" is on.
- The **last selected pane is remembered** (`lastPreferencesPaneIdentifier`) and restored the next time the window opens.
- Escape (cancelOperation) closes the window. The settings window cannot become the main window (does not steal "main" status from document windows).
- Every pane has a round **Help ("?") button** at the bottom-right that opens the help book at an anchor (`settings_general`, `settings_appearance`, `settings_window`, `settings_edit`, `settings_mode`, `settings_format`, `settings_snippets`, `settings_shortcuts`, `settings_integration`). On Linux: open the corresponding help page/URL.
- All controls apply **immediately** (no OK/Apply), backed by user defaults; open document windows update live.
- Layout convention: a two-column grid — right-aligned row labels ending with a colon ("On startup:") in the left column, controls left-aligned in the right column. Rows are separated by vertical spacing (18 pt in General/Edit/Integration, 8 pt in Window) and, in some panes, horizontal dividers.

Pane order (toolbar), label, SF Symbol icon:

| # | Label | SF Symbol | Linux/Adwaita icon suggestion |
|---|---|---|---|
| 1 | General | `gearshape` | `preferences-system-symbolic` / `emblem-system-symbolic` |
| 2 | Appearance | `eyeglasses` | `preferences-desktop-appearance-symbolic` |
| 3 | Window | `uiwindow.split.2x1` | `view-dual-symbolic` / `window-symbolic` |
| 4 | Edit | `square.and.pencil` | `document-edit-symbolic` |
| 5 | Mode | `switch.2` | `toggles-symbolic` |
| 6 | Format | `text.document` | `text-x-generic-symbolic` |
| 7 | Snippets | `text.viewfinder` | `insert-text-symbolic` |
| 8 | Shortcuts | `keyboard` | `input-keyboard-symbolic` |
| 9 | Integration | `apple.terminal.fill` | `utilities-terminal-symbolic` |

GTK note: an `AdwPreferencesWindow`/`AdwPreferencesDialog` with pages gives the same "one pane per tab" model; a toolbar-style `AdwViewSwitcher` in the header bar is closest to the macOS look.

Common list-editing pattern used in several panes (Appearance theme list, Format syntax list, Mode list): a bordered list with rounded 7 pt corners and a **bottom bar** containing borderless icon buttons **+** ("Add", tooltip "Add new item"), **−** ("Delete", tooltip "Delete selected items"), optionally a pencil ("Edit", tooltip "Edit selected item"), a spacer, and an **ellipsis-circle "Actions" menu button** (⋯). The same items are offered in the row context menu (short labels) and in the ⋯ menu (labels include the item name, e.g. `Duplicate “Anura”`). Deletion always asks: title `Are you sure you want to delete “NAME”?`, message `This action cannot be undone.`, buttons Delete (destructive) / Cancel. Import of an existing name asks `“NAME” already exists. Do you want to replace it?` with Replace / Cancel. New items are named `Untitled`, then `Untitled 2`, … (unique numbering); duplicates get the source name plus a unique number.

---

### 1. General

| Row label | Control | Label / options | Default | Notes |
|---|---|---|---|---|
| On startup: | Checkbox | `Reopen windows from last session` | **on** (`NSQuitAlwaysKeepsWindows`) | The first time it is toggled in a session, a dialog appears: title `The change will be applied first on the next launch.` message `The app’s behavior will change from the next launch after that one.` (OK only). Shown once per settings session. |
| (same row, below) | Static text + popup menu (indented 20 pt) | Caption `When nothing else is open:`; popup items: `Create New Document`, `Show Open Dialog`, `Do Nothing` | `Create New Document` | Behaviour at launch / on reopen (dock click) when no windows are open. |
| Document save: | Checkbox with subtitle | `Enable Auto Save with Versions`; subtitle (secondary, small): `A system feature that automatically overwrites your files while editing. Even if turned off, CotEditor covertly creates a backup in case it unexpectedly quits.` | **on** | Changing it (relative to the value when the pane opened) shows a confirmation dialog: title `The change will be applied first on the next launch.`, message `Do you want to restart CotEditor now?`, buttons `Restart Now` (relaunches app), `Later`, `Cancel` (reverts the checkbox). **[macOS-only concept: "Versions"]** — on Linux map to "Auto-save in place" (write to the file periodically) vs. "backup only" (autosave to a hidden backup file for crash recovery). |
| `When document is changed by another application:` (label spans both columns) | Radio group (vertical) | `Keep CotEditor’s version`, `Ask how to resolve`, `Update to modified version` | `Update to modified version` | External file change policy: ignore / show a prompt / silently revert (reload) if the document has no unsaved conflicts. Linux: GFileMonitor-based. |
| Settings import & export: | Checkbox with subtitle | `Keep a copy in iCloud`; subtitle `Store a copy of these settings for other devices to import.` | off | **[macOS-only: iCloud]**. If the iCloud manager has an error, a small warning label (triangle icon + error text) appears under it. Linux: omit, or replace with "Keep a copy in a sync folder". |
| (same row) | Button | `Export…` | — | Opens the **Export Settings** sheet (see below). |
| (same row) | Pull-down button (menu button with primary action) | `Import…` | — | Clicking the button body opens a file chooser directly (message `Choose a CotEditor settings archive to import.`, confirm button `Import`, file type `.cotsettings`). The menu arrow lists: iCloud copies (each `MACHINE — USER` + secondary `DATE (iCloud)`), or `Loading Settings Copies from iCloud…`, or `No Settings Copies in iCloud`, or the iCloud error text; separator; `Manage Copies on iCloud…` (only if copies exist); `Choose Settings Archive…`. A small spinner shows while an iCloud copy loads. iCloud parts **[macOS-only]**. |
| Dialog warnings: | Button | `Manage Warnings…` | — | Opens a small sheet: text `Suppress the following warnings:`, checkbox `Inconsistent line endings` (default off), Help button, `Done` (default button). |
| Software update: *(only in the non-App-Store build with Sparkle; preceded by a divider)* | Checkboxes | `Check for updates automatically` (default on); `Update to prereleases when available` (default off); if the running version is a prerelease, small secondary note: `Regardless of this setting, new prereleases are always included while using a prerelease.` | | **[macOS-only: Sparkle]** — Linux: omit (distribution package manager / Flatpak handles updates). |

Locale detail: for German, French, Italian and Spanish the long label "Settings import & export:" is placed on its own full-width row above the controls instead of in the label column. (Same trick as the external-change label.)

**Export Settings sheet** (width 420): title `Export Settings`; small text `Export selected settings as an archive to allow importing into other computers.`; a grouped form of checkboxes with subtitles:
- `Settings` — `All settings in the Settings window` (always shown)
- `Themes` — `N custom themes` (only if user themes exist)
- `Syntaxes` — `N custom syntaxes`
- `Multiple Replace Definitions` — `N definitions`
- `File Scopes` — `N items` (saved folder-find scopes)

Bottom: Help button, Cancel, `Export…` (default; disabled if nothing checked) → save dialog for a `.cotsettings` archive.

**Import Settings sheet** (width 420): title `Import Settings`; if the archive's version differs, a yellow-bordered warning box: `This settings archive was created by a different version of CotEditor (VERSION).` + `You can still import these items, but some settings may be incompatible.`; text `Select the items to import:`; the same checkbox list (only types present in the archive; all checked by default); buttons Help / Cancel / `Import…`.

**Manage Copies on iCloud sheet** **[macOS-only]**: list of copies (name = `machine — user`, sub line `DATE, Version X`), current machine tagged with a small outlined `This Mac` badge; overlay messages for loading (spinner), error, or `No Settings Copies in iCloud`; button `Delete Copy` (destructive, confirmation `Are you sure you want to delete “NAME” from iCloud?` / `This action cannot be undone.`); note `To stop saving a copy, turn off “Keep a copy in iCloud” on the Mac it comes from.`; Help + Close.

---

### 2. Appearance

Top section is a form (right-aligned labels), followed by the theme list/editor, then the Help button.

| Label | Control | Details | Default |
|---|---|---|---|
| Standard font: | Font row | A text label showing the font rendered in itself as `DisplayName  Size` (max width 260, tooltip same text, rendered without antialiasing if Antialias is off); a **font size stepper** (accessibility value "N points"); a `Select…` button that opens the system font picker. Below, two small checkboxes: `Antialias`, `Ligatures`. | System user font, 13 pt; Antialias **on**; Ligatures **on** |
| Monospaced font: | Same font row | Same controls. If the user picks a font that is not fixed-pitch, an alert: title `The selected font doesn’t seem to be monospaced.` message `Do you want to use it for the monospaced font?`, buttons OK (keep) / Cancel (revert to previous font). | System fixed-pitch font (SF Mono/Menlo), 13 pt; Antialias **on**; Ligatures **off** |
| Line height: | Stepper with numeric field + unit text `times` | range 0.1–10, step 0.1, 1–2 decimals | **1.3** |
| Appearance: | Horizontal radio group | `Match System`, `Light`, `Dark` | `Match System` — forces the document windows' light/dark appearance. |
| Status bar: | Horizontal radio group | `Tinted`, `Opaque` | `Tinted` (prefersOpaqueBarBackground = false) — background style of the window's bars (status/navigation). |
| Editor opacity: | Slider + percent text field | Slider range 20 %–100 % with tick marks every 10 %, min label is a small transparent swatch (tooltip `Transparent`), max label an opaque swatch (tooltip `Opaque`), current value shown as percent; text field (width 64, placeholder `100%`) mirrors the value; values are clamped to 0.2–1.0. | **100 %** — sets the editor background alpha (window transparency). |

Font picker on Linux: `GtkFontDialogButton` or a label + `Select…` button opening `GtkFontDialog`; keep the separate size stepper.

#### Theme list and editor (embedded, rounded bordered box, horizontal split)

**Left: theme list** — header `Theme`; one row per theme (bundled + user). Bundled themes: `Anura`, `Anura (Dark)`, `Classic`, `Classic (Dark)`, `Dendrobates`, `Dendrobates (Dark)`, `Kawazu`, `Lakritz`, `Mono`, `Note`, `Printen`, `Pulse`, `Resinifictrix`, `Resinifictrix (Dark)`. Default theme: **Anura**.
- Rows are inline-renamable text fields (double-click / "Rename" puts the row in edit mode; bundled themes cannot be renamed).
- Selecting a row **immediately makes it the default theme**. A theme is "dark" if its name ends with `(Dark)`. When not pinned, CotEditor automatically uses the `X (Dark)` sibling in dark appearance and `X` in light. If the user selects a theme whose darkness differs from the current appearance, `pinsThemeAppearance` becomes true and the chosen theme is used regardless of appearance (no auto-switch); selecting one that matches clears the pin.
- Drag & drop: dropping `.cottheme` files from Finder imports them; dragging a user theme out exports the file; dragging to the trash deletes (bundled themes excluded).
- Bottom bar: **+** creates `Untitled` theme (default colors) and selects it; **−** deletes the selected theme (disabled for bundled ones; confirmation dialog; afterwards the default theme setting is reset); ⋯ Actions menu.
- ⋯ Actions menu (for selection NAME): `Duplicate “NAME”`; `Rename “NAME”` (disabled for bundled); `Restore “NAME”` (enabled only for a bundled theme that the user modified — reverts to factory); `Export “NAME”…` (save as `.cottheme`; with Option held it becomes `Reveal “NAME” in Finder`; disabled unless the theme is user-customized); `Share…` (macOS share sheet, **[macOS-only]**); separator; `Import…` (file chooser, multiple `.cottheme`; with Option held: `Reload All Themes`). Context menu on a row: `Duplicate`, `Rename`, `Restore`, `Delete`, `Export…` / `Reveal in Finder`, `Share…`.

**Right: theme editor** — color wells in two columns, labels right-aligned:
- Column 1: `Text:` (no alpha), `Invisibles:`, `Cursor:` (with checkbox `Use system color` below; when checked the well is disabled and shows the system insertion-point color; no alpha).
- Column 2: `Background:` (no alpha), `Current Line:`, `Selection:` (+ `Use system color`; system = selected-text background color), `Highlight:` (+ `Use system color`; system = accent color; used for find/brace/instance highlights).
- Bold heading `Syntax`, then column 1: `Keywords:`, `Commands:`, `Types:`, `Attributes:`, `Variables:`; column 2: `Values:`, `Numbers:`, `Strings:`, `Characters:`, `Comments:`.
- Bottom-right borderless circled-"i" button (`Show theme file information`) opens a popover with fields `Author`, `URL` (with a link/open button), `License`, `Description` (multi-line 2–5). Placeholder `Not defined`. Editable for user themes, read-only labels for bundled themes.
- Every color change is saved instantly to the theme file. Editing a bundled theme silently creates a user-customized copy with the same name (which then makes `Restore` available).
- Untitled theme defaults: text = system text color, background = system text background, invisibles = 70 % gray, current line = 95 % gray, selection/cursor/highlight = "use system color", all syntax colors = gray.

---

### 3. Window

| Row label | Control | Options / label | Default | Notes |
|---|---|---|---|---|
| Open documents in tabs: | Popup | `Respect System Setting (CURRENT)` — the current system value shown in secondary color — then separator, `Never`, `Automatically`, `Always` | `Respect System Setting` (-1) | Window tabbing mode. **[macOS-specific system setting]** — on Linux "Respect System Setting" can map to an app default (e.g. Always/tabs). "Automatically" = tabs only in full screen. |
| Window size: | Two numeric fields with captions `Width` / `Height` below and unit `px` after each | placeholder `Auto` when empty | empty (Auto) | Initial size of new document windows. |
| — divider — | | | | |
| Show: | Checkbox | `Line numbers` | **on** | |
| | Checkbox | `Invisible characters` | off | Master toggle for invisible display (also View menu). |
| | 3+2 grid of checkboxes (always enabled, independent of master) | Row 1: `Line ending` (on), `Tab` (on), `Space` (off); Row 2: `Other whitespace` (on), `Other control characters` (on, spans 2 cols) | as listed | Which invisible types are drawn when invisibles are shown. |
| | Checkbox | `Indent guides` | off | |
| | Checkbox + stepper number field | `Page guide at column:` [80] | off; column **80** (range 1–999) | Field disabled while checkbox is off. |
| Current line: | Checkbox | `Change background color` | off | Current-line highlight. |
| — divider — | | | | |
| Line wrapping: | Checkbox | `Wrap lines to editor width` | **on** | |
| | Checkbox + stepper field + text `spaces` | `Indent wrapped lines by` [0] `spaces` | **on**, width **0** (range 0–99) | Hanging indent for wrapped lines (relative to the line's own indentation). Field disabled when off. |
| Writing direction: | Horizontal radio group | `Left to right`, `Right to left`, `Vertical` | `Left to right` | Default for new documents. |
| Overscroll: | Stepper with percent field | 0 %–100 %, step 10 % | **0 %** | Extra scrollable space after the last line, as a fraction of the view height. |
| — divider — | | | | |
| Status bar shows: | Two columns of checkboxes | Col 1: `Line count` (**on**), `Character count` (**on**), `Word count` (off); Col 2: `Location` (**on**), `Current line` (**on**), `Current column` (off) | as listed | Which counters appear in the status bar. |

StepperNumberField = a small numeric text field with an attached up/down stepper; empty input reverts to the default value.

---

### 4. Edit

| Row label | Control | Options / label | Default | Notes |
|---|---|---|---|---|
| Indentation: | Popup with leading label | `Prefer using` [`Spaces` / `Tabs`] | **Tabs** (autoExpandTab = false) | Default indent style. |
| | Label + stepper field + unit | `Indent width:` [4] `spaces` | **4** (1–99) | Tab width / indent width. |
| | Checkbox | `Detect indent style on document opening` | **on** | Guess tabs/spaces from file content. |
| | Checkbox | `Automatically indent while typing` | **on** | Copies indentation on Return (and uses syntax block delimiters for smart indentation). |
| | Checkbox | `Indent selection with Tab key` | off | Tab/Shift-Tab shift multi-line selections. |
| Whitespace: | Checkbox | `Automatically trim trailing whitespace` | off | Trims automatically 3 s after the last text change (debounced), keeping whitespace at the insertion point(s) being edited. |
| | Checkbox (indented 20 pt) | `Including whitespace-only lines` | off | Disabled unless the trim option is on. |
| Comment: | Checkbox | `Insert comment delimiters after indent` | **on** | Comment-out puts `//` after leading indentation instead of column 0. |
| | Checkbox | `Add a space to comment delimiters` | **on** | `// foo` vs `//foo`. |
| Content parse: | Checkbox | `Link URLs in document` | off | Clickable URL detection. |
| | Checkbox | `Highlight matching braces` | **on** | |
| | Checkbox | `Highlight instances of selected text` | **on** | |
| | Small label + stepper + unit (indented, small size) | `Delay:` [0.50] `seconds` | **0.5 s** (0–10, step 0.25, 2 decimals) | Disabled when instance highlighting is off. |

---

### 5. Mode

Purpose: per-"mode" editor behaviour. A **mode** is either a syntax **kind** (`General` for prose-like syntaxes, `Code` for programming syntaxes) or a specific syntax. Resolution for a document: if a syntax-specific mode exists for its syntax → use it; otherwise use the mode of the syntax's kind (default `General`).

Layout: left = bordered list; right = a group box with the options form.

**Left list**
- Section header `Mode`: rows `General`, `Code` (always present).
- Section header `Syntax` (only if any): one row per syntax-specific mode. If the syntax no longer exists, the row shows a trailing warning icon and tooltip `This syntax does not exist`, and the options box is disabled.
- Bottom bar: **+** is a pull-down menu with section `Syntax` listing all syntaxes (those already added are disabled); choosing one creates a syntax-specific mode (initialized from the current resolved options) and selects it. **−** deletes the selected syntax mode (disabled for `General`/`Code`); selection returns to `General`. No confirmation.
- Context menu on `General`/`Code`: `Restore` (resets to factory defaults; disabled when already default).

**Right: options form** (changes saved immediately)

| Label | Control | Items |
|---|---|---|
| Font: | Horizontal radio group | `Standard`, `Monospaced` (which of the two Appearance fonts to use) |
| Substitution: | Checkboxes | `Smart copy/paste`, `Smart quotes`, `Smart dashes`, `Text replacement`, `Add period with double-space`, `Automatically insert closing brackets and quotes` |
| Spelling: | Checkboxes | `Check spelling while typing`, `Check grammar with spelling`, `Correct spelling automatically` |
| Completion: | Text `Completion list includes:` then indented checkboxes `Standard words`, `Words in document`, `Words defined in syntax`; then checkbox `Suggest completions while typing` (disabled if no word source is checked) | |
| Indentation: | Checkbox `Use custom settings`; indented (disabled unless checked): `Prefer using` popup [`Spaces`/`Tabs`], `Indent width:` stepper field + `spaces` | Overrides the Edit-pane defaults; when turned on it is initialized from the Edit-pane values. |

Factory defaults:

| Option | General | Code |
|---|---|---|
| Font | Standard | Monospaced |
| Smart copy/paste | on | off |
| Smart quotes | off | off |
| Smart dashes | off | off |
| Text replacement | on | on |
| Add period with double-space | off | off |
| Auto-close brackets and quotes | off | **on** |
| Check spelling while typing | **on** | off |
| Check grammar with spelling | off | off |
| Correct spelling automatically | off | off |
| Completion sources | Standard words + Words in document | Words in document + Words defined in syntax |
| Suggest completions while typing | off | off |
| Custom indentation | off | off |

Notes: "Smart quotes/dashes", "Text replacement" (system text replacements), "Add period with double-space", grammar checking and "Standard words" (system dictionary completion) are **macOS text-system features** — on Linux they need own implementations (or omit grammar/text-replacement; spelling via libspelling/enchant).

---

### 6. Format

| Row label | Control | Options | Default | Notes |
|---|---|---|---|---|
| Default line endings: | Popup | `macOS / Unix (LF)`, `Classic Mac OS (CR)`, `Windows (CRLF)` | **LF** | For new documents. |
| — divider — | | | | |
| Default text encoding: | Popup (flexible width) | The encoding list in its user order, separators between groups; UTF-8 appears twice: `Unicode (UTF-8)` and `Unicode (UTF-8) with BOM` | **Unicode (UTF-8)** (no BOM) | For new documents. |
| Encoding priorities: | Button | `Edit List…` | — | Opens the **Encoding List** sheet. (The app-level action "show encodings list editor" — used by the Encoding menu's customize item — opens Settings on the Format pane and presents this sheet automatically.) |
| | Checkbox | `Refer to encoding declaration in document` | **on** | Use `charset=`/`encoding=`/`-*- coding -*-` declarations when detecting encoding on open. |
| — divider — | | | | |
| Default syntax: | Popup | `None`, separator, then all syntax names (if the stored default no longer exists it is listed with tooltip `This syntax does not exist`) | `Plain Text` | Syntax for new documents. |
| Available syntaxes: | List (top-aligned) | see below | | |

**Syntax list**: plain list of all syntaxes (bundled + user). Each row has a small leading dot that is visible only when the syntax is user-customized (tooltip `This syntax is customized.`). Double-click (primary action) opens the editor sheet. Customized syntaxes can be dragged out (exports file) or dragged to the trash (deletes). Dropping `.cotsyntax`/`.yaml` files imports them.
- Bottom bar: **+** (new syntax → editor sheet with empty definition), **−** (delete; disabled for bundled; confirmation), **pencil** (edit selected), spacer, ⋯ Actions.
- ⋯ Actions menu: `Duplicate “NAME”`, `Restore “NAME”` (only for modified bundled), `Export “NAME”…` (Option: `Reveal “NAME” in Finder`; only if customized), `Share…` **[macOS-only]**, separator, `Import…` (Option: `Reload All Syntaxes`), separator, `Customize Syntax Menu…`, `Show File Mapping Conflicts` (disabled if none).
- Context menu: `Edit…`, `Duplicate`, `Restore`, `Delete`, `Export…`/`Reveal in Finder`, `Share…`.

**Encoding List sheet**
- Text `Drag encodings to change the order:`; a reorderable list (drag to move, Delete key removes selection). Rows show `Localized name` + IANA charset name (secondary, e.g. `utf-8`), separators shown as dividers. UTF-8 is shown as two rows: `Unicode (UTF-8)` and `Unicode (UTF-8) with BOM`.
- Below the list: a + / − control group. **+** is a menu: `Separator`, then section `Text Encoding` with every available encoding not already listed (sorted by name); inserted after the selection. **−** removes selection; disabled with tooltip if it would remove UTF-8 (`Unicode (UTF-8) can’t be removed.`) or the default encoding (`The encoding set as default can’t be removed.`).
- Footnote (small): `This order is used for the Encoding menu and for encoding detection. Only the encodings listed here are considered during detection, with higher items taking priority.`
- Buttons: Help, `Restore Defaults` (enabled when differs from factory), Cancel, OK (saves). Undo/redo supported inside the sheet.
- Factory order (separators = —): UTF-8 — Shift JIS, EUC-JP, Japanese (Windows, DOS), Shift JIS X0213, Japanese (Mac OS), ISO-2022-JP — Mac Roman, Windows Latin 1 — GB18030, Big5-HKSCS, Big5-E, Big5, Trad. Chinese (Mac OS), Simp. Chinese (Mac OS), EUC-TW, GB 2312 (EUC-CN), Trad. Chinese (Windows, DOS), Simp. Chinese (Windows, DOS) — Korean (Mac OS), EUC-KR, Korean (Windows, DOS) — Thai (Windows, DOS), Thai (ISO 8859-11) — Arabic (Mac/ISO 8859-6/Windows), Greek (Mac/ISO 8859-7/Windows), Hebrew (Mac/ISO 8859-8/Windows), Cyrillic (Mac/ISO 8859-5/Windows), Russian (DOS), Central European (Mac OS), Turkish (Mac OS), Icelandic (Mac OS) — ISO Latin 1–10 (8859-1,2,3,4,9,10,13,14,15,16) — Latin-US (DOS), Central European (Windows Latin 2), Western (NextStep), Western (ASCII) — UTF-16, UTF-16BE, UTF-16LE, UTF-32, UTF-32BE, UTF-32LE. (Mac-specific encodings like Mac Roman exist in iconv as MACINTOSH etc.; NextStep may be dropped on Linux.)

**Customize Syntax Menu sheet**: text `Select the syntaxes you want to appear in syntax menus:`; a list where each syntax has a checkbox (checked = visible). Toggling a checkbox of a selected row applies to all selected rows. Button `Select All` (disabled if nothing hidden). Footnote `Hidden syntaxes are still used for automatic detection.` Buttons Help / Cancel / OK. Default: none hidden.

**Syntax Mapping Conflicts sheet**: heading `Syntax Mapping Conflicts`; text `The following file mapping rules are registered in multiple syntaxes. CotEditor uses the first syntax automatically. To resolve conflicts, edit each syntax definition.`; up to three sortable bordered tables titled `Extension`, `Filename`, `Interpreter`, each with columns [Extension|Filename|Interpreter], `Used syntax` (bold), `Duplicated syntaxes` (comma list). Help + Close (Esc closes).

**Syntax editor sheet** (overview): a navigation split view.
- Sidebar sections: **Features** — `File Mapping`, `Delimiters`, `Outline`, `Completion`; **Highlighting** — `Keywords`, `Commands`, `Types`, `Attributes`, `Variables`, `Values`, `Numbers`, `Strings`, `Characters`, `Comments`; **Definition File** — `Information`, `Validation` (with a prominent red badge = error count). For syntaxes parsed by the built-in tree-sitter parser, Outline and/or all Highlighting entries are replaced by a locked row (pencil-slash icon) / a `Built-in` pane showing `Managed by the app` and `This language uses a structure-based, general-purpose parser called tree-sitter for syntax analysis. Because the extraction rules are managed by the app, you can’t customize them.`
- Header above detail: name text field (placeholder `Syntax name`; bundled syntaxes show the name as bold read-only text, tooltip `Built-in syntaxes can’t be renamed.`); inline name-validation error with arrow icon; right-aligned `Kind:` popup (`General` / `Code`).
- Panes: **File Mapping** — three editable lists `Extensions:` `(without dot)`, `Filenames:`, `Interpreters:` (note `The interpreters are used to determine the syntax from the `#!` header in the document.`). **Delimiters** — `Inline comment:`, `Block comment:` (begin/end), table `Block delimiters:`/`Indentation` with note `The block delimiters are used for automatic indentation while typing.`, `String delimiters:` and `Character delimiters:` tables (columns `Begin String`, `End String`, `Escape Character`, `Multiline`, `Prefixes`, `Description`; block comments also `Nest`, `Line Start Only`), note `The delimiters defined here are used for syntax highlighting as well.` **Outline** — table `Regular Expression Pattern`, `IC` (ignore case), `Display Pattern`, `Kind` (None, Container, Value, Function, Title, Heading 1…, Mark, Separator), `Description`. **Completion** — table `Completion`, `Type`; note `If not specified, syntax completion words are generated based on the highlighting settings.` **Highlight panes** — table `Begin String`, `End String`, `RE` (regex), `IC`, `Multiline`, `Description`; regex cells show validity marks. **Information** — `Version:`, `Last Modified:`, `Distribution URL:`, `Author:`, `License:`, `Description:`. **Validation** — `No errors found.` or `N errors found.` list (e.g. `Invalid regular expression.`, `Block comment needs both begin and end delimiters.`, `The same word is registered multiple times.`, `Nestable block comment must use different begin and end delimiters.`, `The escape character must be a single character in UTF-16 code unit.`). Validation re-runs 0.3 s after each edit.
- Bottom: Cancel / OK. OK trims the name, validates it (beeps and focuses name on error), validates the definition (beeps and jumps to Validation on errors), then saves.

---

### 7. Snippets

A tab view with two tabs: `Command` and `File Drop`. Below: Help button.

**Command tab** — text `Text to be inserted by a command in the menu or by keyboard shortcut:`
- Bordered table, rows draggable to reorder, multi-select. Columns:
  - `Syntax` (width 160): borderless popup `All` / separator / syntax names — scope where the snippet is available.
  - `Name`: inline text field — the menu item title.
  - `Key` (width 80): a shortcut-recorder field; click and press keys. Validation errors (e.g. already used) appear as small red text next to the +/− buttons.
- Below table: +/− buttons (+ adds `Untitled` / `Untitled 2` …).
- `Insertion format:` label with a right-aligned `Insert Variable` menu button, then a token text editor (rounded border) for the selected row. Disabled with overlay text `No item selected` or `Multiple items selected` unless exactly one row is selected. Variables (displayed as tokens, stored as `<<<NAME>>>`): `SELECTION` — `The selected text.`; `CURSOR` — `The insertion point after inserting the snippet.`
- Snippets appear in the app's Snippets menu (general ones, then those scoped to the current syntax). Default: no snippets.

**File Drop tab** — text `Text to be inserted by dropping files to the editor:`
- Bordered table, draggable rows. Columns: `Syntax` (popup, tooltip `Syntax in which this file drop setting is used.`), `Extensions` (comma-separated text field, placeholder `All`, tooltip `File extensions of dropped file (comma-separated).`), `Description` (text).
- Below: +/− and `Restore Defaults` (enabled when list differs from factory).
- `Insertion format:` + `Insert Variable` menu: path tokens `ABSOLUTE-PATH`, `RELATIVE-PATH`, `FILENAME`, `FILENAME-NOSUFFIX`, `FILEEXTENSION`, `FILEEXTENSION-LOWER`, `FILEEXTENSION-UPPER`, `DIRECTORY`; section `Text File`: `FILECONTENT`; section `Image File`: `IMAGEWIDTH`, `IMAGEHEIGHT`. Each menu item shows the token and a description (e.g. `The relative path between the dropped file and the document.`).
- Entries with empty format are dropped on save. First matching rule (by syntax and extension) wins.
- Factory rules:
  1. Markdown, jpg/jpeg/gif/png → `![<<<FILENAME-NOSUFFIX>>>](<<<RELATIVE-PATH>>>)`
  2. Markdown, all → `[<<<FILENAME-NOSUFFIX>>>](<<<RELATIVE-PATH>>>)`
  3. HTML, jpg/jpeg/gif/png → `<img src="<<<RELATIVE-PATH>>>" alt="<<<FILENAME-NOSUFFIX>>>" title="<<<FILENAME-NOSUFFIX>>>" width="<<<IMAGEWIDTH>>>" height="<<<IMAGEHEIGHT>>>" />`
  4. HTML, js → `<script type="text/javascript" src="<<<RELATIVE-PATH>>>"></script>`
  5. HTML, html/htm/php → `<a href="<<<RELATIVE-PATH>>>" title="<<<FILENAME-NOSUFFIX>>>"></a>`
  6. CSS, all → `url("<<<RELATIVE-PATH>>>")`

---

### 8. Shortcuts

- Text: `To change a shortcut, click the key column, then type the new keys.`
- A bordered box containing a 2-column table without column headers (`Command` | `Key` width 120), with a **tab switcher at its top-left**: two pill buttons `Menu` and `Text Editing` (selected pill gets a filled rounded background; on older macOS a segmented control).
- **Menu** category: an outline (tree) of the whole main menu — one section per top-level menu (CotEditor/App, File, Edit, View, Format, Find, Text, Window, Help…), expandable submenus. Commands whose shortcut differs from default are shown **bold**. The Key column is a shortcut recorder.
  - Excluded from customization: separators, alternate items, hidden items, items with Fn modifier, the Services submenu, dynamic lists (encodings, syntaxes, themes, line endings, snippets, scripts, help anchors, documents in Window menu), Save As, Help, the Multiple Replace find action, and any item not present at launch.
- **Text Editing** category: sections `Movement`, `Selection`, `Deletion`, `Editing`, `Scrolling` listing text-system commands; each command can have **several shortcuts** — the key cell shows the existing shortcuts separated by `, ` plus an extra empty recorder slot to add another. Commands with customized bindings are bold.
  - Movement: Move Forward (⌃F), Move Backward (⌃B), Move Up (⌃P), Move Down (⌃N), Move Word Forward (⌃⌥F), Move Word Backward (⌃⌥B), Move Word Left (⌥←), Move Word Right (⌥→), Move to Left End of Line (⌘←, ⌃←), Move to Right End of Line (⌘→, ⌃→), Move to Beginning of Paragraph (⌃A), Move to End of Paragraph (⌃E), Move Page Up (⌥PageUp), Move Page Down (⌥PageDown, ⌃V), Move to Beginning of Document (⌘↑), Move to End of Document (⌘↓).
  - Selection ("… Extending Selection"): Forward ⌃⇧F, Backward ⌃⇧B, Left ⇧←, Right ⇧→, Up ⇧↑/⌃⇧P, Down ⇧↓/⌃⇧N, Word Forward ⌃⌥⇧F, Word Backward ⌃⌥⇧B, Word Left ⌥⇧←, Word Right ⌥⇧→, Left End of Line ⇧⌘←/⇧⌃←, Right End of Line ⇧⌘→/⇧⌃→, Beginning of Paragraph ⌃⇧A, End of Paragraph ⌃⇧E, Paragraph Backward ⌥⇧↑, Paragraph Forward ⌥⇧↓, Page Up ⇧PageUp, Page Down ⇧PageDown/⌃⇧V, Beginning of Document ⇧⌘↑/⇧Home, End of Document ⇧⌘↓/⇧End.
  - Deletion: Delete Word Backward (⌥⌫, ⌃⌥⌫), Delete Word Forward (⌥⌦), Delete to Beginning of Line (⌘⌫), Delete to End of Line (—), Delete to Beginning of Paragraph (—), Delete to End of Paragraph (⌃K).
  - Editing: Transpose (⌃T), Yank (⌃Y).
  - Scrolling: Scroll Line Up (—), Scroll Line Down (—), Scroll Page Up (PageUp, ⌃↑), Scroll Page Down (PageDown, ⌃↓), Scroll to Beginning of Document (Home), Scroll to End of Document (End).
  - These are Emacs/Cocoa text bindings — on Linux the defaults should be adapted to GTK/PC conventions (Ctrl+←/→ word, Home/End line, Ctrl+Home/End document) while keeping the customization model.
- Validation errors shown as small text at the bottom: `Single type is invalid for a shortcut.`, `“KEY” is already taken by the “COMMAND” command.`, `The Shift key can be used only with another modifier key.`, `The combination “KEY” is not supported for the shortcut customization.`
- Context menu on a row: `Restore Defaults` (resets that command; disabled if unchanged). Delete key on a selected row also restores it.
- Bottom bar: `Restore Defaults` (all; enabled when anything is customized), error text, Help. Undo/redo supported for shortcut edits.

---

### 9. Integration

| Row label | Content | Default |
|---|---|---|
| Command-line tool: | Button `Learn More…` (opens help "about_cot"); if the `cot` symlink is installed, a label `installed at PATH` with a status dot (green = valid, yellow = links to a different CotEditor with tooltip `The current `cot` symbolic link doesn’t target the running CotEditor.`, red = broken with tooltip `The current `cot` symbolic link may target an invalid path.`). Small text: `With the `cot` command-line tool, you can launch CotEditor and let it open files from the command line.` | — |
| Remote editing: | Checkbox `Accept rmate connections`; `Port:` number field (no grouping, placeholder 52698; disabled when unchecked); error label with warning icon if the server can't start; small text `Allow remote files to be opened in CotEditor using the `rmate` protocol.` (the word `rmate` is a help link). | off; port **52698** |

Linux notes: the `cot` tool install via symlink in `/usr/local/bin` **[macOS-specific install flow]** — on Linux ship a `bin/` launcher (or install to `~/.local/bin`) and show its status. rmate server is platform-neutral (TCP).

---

### Other persisted defaults relevant to Settings (no UI in the Settings window)

Stored but edited elsewhere (menus, panels): `showStatusBar` (on), `showNavigationBar` (on), `splitViewVertical` (off), `selectedInspectorPaneIndex` (0), `fileBrowserShowsHiddenFiles` (off), outline font size / wrapping, console & find-result font size (small system size), `colorCodeType` (1), `recentSyntaxNames` (max 6), `largeFileAlertThreshold` (200 MB), `savesTextOrientation` (on), find options (regex off, ignore case off, wrap on, in selection off, incremental on, regex multiline on, unescape replacement on…), character count options (unit = grapheme cluster, NFC, etc.), print settings (header: file path left / print date right; footer: page number center; print background off; header & footer on), `hiddenSyntaxes` (empty), `donationBadgeType`.

### macOS-only summary for this window
- iCloud "Keep a copy" + Manage Copies sheet + iCloud section of Import menu → omit or replace with a sync-folder option.
- "Enable Auto Save with Versions" naming and restart prompt → keep as an "auto-save in place" toggle; restart prompt optional.
- "Respect System Setting" for window tabbing → app-level default.
- Sparkle update section → omit.
- Share… menu items, "Reveal in Finder" (→ "Show in Files" via `org.freedesktop.FileManager1`).
- `cot` symlink installer → Linux launcher script.
- Mode options relying on the Cocoa text system (smart quotes/dashes, text replacement, double-space period, grammar check, "Standard words" completion) → need own implementations or omission.


## macOS-only Features and Linux Substitutes

This section consolidates the **[macOS-only]** markers from the whole document.

| Feature | Where | Linux suggestion |
|---|---|---|
| Services menu (app menu; file browser context menu; text Services) | CotEditor menu, File Browser | Omit. |
| Hide / Hide Others / Show All / Bring All to Front / Minimize | App and Window menus | Omit (Minimize optional). |
| Dock menu "New Document" | Dock | `.desktop` file action "New Window". |
| Versions browser ("Revert To ▸ Browse All Versions…"), proxy-icon title menu (Rename/Move/Lock) | File menu, title bar | Keep "Revert to Saved". Add Rename…/Move To… to the File menu if wanted. Optionally keep timestamped backups. |
| "Enable Auto Save with Versions" | Settings → General | Rename to "Auto-save in place" (write the file about 5 s after typing stops). The off-state keeps a crash-recovery backup in `$XDG_STATE_HOME`. |
| Locked files (Finder "Locked") unlock/duplicate flow | Document Lifecycle | For non-writable files: open read-only and offer "Save As…" / "Duplicate". |
| iCloud settings copies, Manage Copies on iCloud | Settings → General | Omit, or "keep a copy in a folder". |
| Sparkle "Check for Updates…", update settings | App menu, Settings → General | Omit. |
| Donation / tip jar, status-bar donation badge | App menu, status bar | Omit or link to a web page. |
| AppleScript / JXA scripts, AppleScript Dictionary, script event hooks (document opened/saved) | Scripts, Help menu | Unix executables only. Optional hooks folder (e.g. `scripts/_hooks/on-open`, `on-save`). |
| Script folder path `~/Library/Application Scripts/com.coteditor.CotEditor/` | Scripts | `$XDG_DATA_HOME/<app>/scripts/`. |
| Shared system find pasteboard (find string synced across apps) | Find panel | Keep the find string app-global only. |
| System font panel ("Show Fonts") | Format → Font, toolbar Fonts | `GtkFontDialog`. |
| System colour panel ("Edit Color Code…") | Text menu | `GtkColorChooserDialog`/popover with the same code field, type popup and Insert button. |
| Emoji & Symbols palette (Edit menu, toolbar item) | Edit menu, toolbar | GTK emoji chooser (Ctrl+.), or the app's Unicode input popover. |
| Spelling and Grammar panel, grammar checking | Edit menu | libspelling (GtkSourceView), spelling only. |
| Substitutions (smart quotes/dashes, Text Replacement, Show Substitutions, Smart Copy/Paste) | Edit menu, Mode settings | Implement smart quotes/dashes only if desired; omit Text Replacement and the panel. |
| Speech (Start/Stop Speaking) | Edit menu | Omit (or speech-dispatcher). |
| Insert from iPhone or iPad (Continuity Camera, Live Text) | Edit menu, context menu | Omit. |
| Writing Tools (Apple Intelligence) | Toolbar, context menu | Omit. |
| AutoFill, Start Dictation (system-added Edit items) | Edit menu | Omit. |
| Share… (file share sheet) | File menu, toolbar, file browser, settings lists | Omit, or use an "Open With…" portal. |
| Show in Finder / Reveal in Finder | File menu, file browser, theme/syntax lists | "Show in Files" via D-Bus `org.freedesktop.FileManager1.ShowItems`. |
| Quick Look (Space in the file browser; preview of non-text files) | File Browser, content preview | Built-in preview: GdkPixbuf/`GtkPicture` for images, `GtkVideo` for media, otherwise a file-info page. |
| Finder tags (Document inspector, file browser dots) | Inspector, File Browser | Omit (or xattr `user.xdg.tags`). |
| `com.apple.TextEncoding` extended attribute (encoding hint read/written) | Open/Save | Optionally use a `user.` xattr. Otherwise rely on BOM, declaration and detection only. |
| `.textClipping` file drop | Editor drop | Omit. |
| Native window tabbing preference ("Respect System Setting", "Automatically" = in full screen) | Settings → Window, File → New Tab, Window menu | `AdwTabView`. Map "Respect System Setting" to an app default. |
| Customize Toolbar palette | View menu | Simple dialog with toggles, or fixed header bar. |
| Touch Bar items | Editor, toolbar | Omit. |
| Keep on Top (floating window level) | Window menu, toolbar | Not available on Wayland. Hide on Wayland, X11 only via `_NET_WM_STATE_ABOVE`. |
| Window opacity / editor opacity (non-opaque window) | Settings → Appearance | GTK can make the editor background translucent with CSS alpha. Compositor support varies, so it may be omitted. |
| Vertical text orientation (tategaki) | Format → Writing Direction, toolbar | GtkTextView/GtkSourceView do not support vertical layout. Omit, or show it disabled. |
| `cot` command-line tool install (symlink in `/usr/local/bin`) | Settings → Integration | Ship a `bin/` launcher or document `flatpak run …`. Single-instance open via `GApplication` command-line/open. |
| Help Book (`CotEditor Help`, help anchors from every "?" button) | Help menu, settings panes, sheets | Open local HTML help or online docs at the matching page. |
| App Store / sandbox security-scoped bookmarks | Throughout | Not applicable. Use portals under Flatpak. |


## Clone Checklist: UX Details That Must Be Right

These are the behaviours that most define how CotEditor feels. Each is specified in detail in the section named in brackets.

1. **Status bar counts** [Status Bar].
   - Format: `Lines: 195 (1)  Characters: 4,512 (12)  Location: 830  Line: 42`. Label in secondary colour, number in primary, locale digit grouping.
   - `(n)` appears only when the selected count is > 0. Selected lines = distinct lines touched by non-empty selections; carets count 0.
   - Total lines exclude the empty line after a trailing newline.
   - Characters are grapheme clusters. Location and Column are 0-based; Line is 1-based. Location, Line and Column all come from the start of the **first** selection.
   - Defaults: Lines, Characters, Location and Line on; Words and Column off.
   - On the right: file size · encoding popup · line-ending popup.
2. **Encoding and line-ending popups in the status bar** [Status Bar, Document Lifecycle].
   - Choosing an encoding asks **Convert / Reinterpret / Cancel**. A lossy convert offers "Change Encoding" and opens the Warnings pane.
   - Choosing a line ending converts the whole buffer as one undo step.
   - NEL/LS/PS are hidden ⌥-alternates.
3. **One line ending per document** [Overview, Editor View]. Typed and pasted text is normalised to the document's line ending; only Paste Exactly bypasses this. Foreign line endings are drawn as **red** invisibles. Mixed line endings found on open trigger a suppressible alert and are listed in the Warnings pane.
4. **Navigation bar per split editor** [Navigation Bar]. Layout: close-split ✕ (only when there are 2 or more editors), the outline ⌃⌄ buttons, the outline popup showing the current item, and the split button on the right; right-clicking the split button toggles orientation. ⌃ selects the previous outline item *before the one containing the caret*; ⌄ selects the next item after it, or the first item when the caret is above all items. Separators are skipped and the heading text is **selected**. Find menu: ⌥⌘↑ / ⌥⌘↓.
5. **Split editors** [Split Editor]. Up to 4 editors on one buffer, each with its own scroll position, selection and zoom. The orientation toggle is global. ⌃\` cycles focus.
6. **Multi-cursor model** [Editor View, Text Commands].
   - Ways to add selections: ⌘-click adds or removes a caret; ⌥-drag makes a rectangular selection; ⌃⇧↑/↓ adds a caret above or below; ⌃⇧L splits a selection into lines; ⌘D adds the next occurrence.
   - Esc collapses to the first caret.
   - Every text command applies to all selections, **except** Sort, Reverse, Shuffle and Sort by Pattern. Those use only the first selection, or the whole document when nothing is selected.
   - Copy/paste between equal numbers of carets keeps the pieces paired.
7. **Gutter mouse behaviour** [Editor View → Line numbers]. Click selects a line (including its line ending), drag selects lines, ⇧-click extends or shrinks, ⌘-click adds a line selection. Numbers are drawn at 40 % opacity, and at 100 % on lines that hold a selection. The gutter is at least 3 digits wide.
8. **Find & Replace panel** [Find & Replace Panel].
   - Window: one floating panel, not attached to a document. It has multi-line find and replace fields with history menus (20 entries), a clear button and a result label ("3/12", "N matches", "Not found", "N replaced").
   - Options: Regular Expression / Ignore Case / In Selection checkboxes and a ⋯ Advanced options popover. Defaults: Wrap on, Incremental on, Anchors match lines on, Unescape replacement on.
   - The panel's **Replace button = Replace and Find Next**.
   - Search feedback: a wrap HUD, and incremental highlighting with a 0.2 s debounce.
   - Find All shows a result table (Line | Matched Text) inside the panel. Clicking a row selects that match; collapsing the table clears the highlights.
   - ⌘E copies the selection into the find field and turns regex **off**.
9. **External file changes** [Document Lifecycle]. The default is **"Update to modified version"**: a silent reload that preserves the selection. "Ask how to resolve" shows **Keep CotEditor's Version / Update**. A byte-for-byte content comparison suppresses false alarms such as touched files or date-only changes. CotEditor's own saves never trigger it.
10. **Auto-save model** [Document Lifecycle, Settings → General]. The default writes the file in place about 5 s after typing stops, with no save prompts. When it is off, CotEditor keeps a hidden crash backup and prompts on close. An empty untitled document never prompts. The setting takes effect only after a restart.
11. **Transient untitled document** [Overview, Document Lifecycle]. The startup blank document is replaced by the first file opened while it is still untouched. Folder windows keep documents with unsaved changes open in the background when the user switches files.
12. **Per-window toggles vs global settings** [Overview → State scope]. Invisibles, wrap, line numbers, guides, tab width, soft tabs, theme and font type are per window and initialised from Settings. Status bar and navigation bar visibility are global. Changing a setting in Settings resets the matching per-window value in every open window.
13. **Toggle-comment semantics** [Text Commands]. ⌘/ uncomments only if *every* line is commented; otherwise it comments by line. The inline delimiter is preferred, placed after the common indent, with a space after it (both on by default). The menu title flips between "Comment Out" and "Uncomment".
14. **Typing aids** [Editor View → Typing behaviours].
    - Auto-indent copies the previous line's indentation, and syntax block tokens drive smart indent and outdent.
    - Brackets and quotes auto-close only before non-word characters. Typing an opener over a selection wraps it. Typing a closer steps over an auto-inserted one. Backspace deletes an empty auto pair.
    - Smart Home cycles: visual line start → first non-blank character → logical line start.
    - Same-word instance highlight appears after 0.5 s. Matching braces flash.
15. **Scripts menu** [Scripts Menu].
    - Discovery: the menu is built from a folder tree. Sub-folders become submenus, `NN)` prefixes set order only, `-` names become separators, and `_` names are hidden. A shortcut is encoded as a second extension (`@`=⌘, `~`=⌥, `$`=⇧, `^`=⌃).
    - Script headers: `%%%{CotEditorXInput=Selection|CurrentLine|AllText}%%%` (stdin; absent = nothing) and `%%%{CotEditorXOutput=ReplaceSelection|ReplaceCurrentLine|ReplaceAllText|InsertAfterSelection|AppendToAllText|NewDocument|Pasteboard}%%%` (absent = stdout discarded). The document path is passed as `$1`.
    - Any stderr output goes to the Console and opens it.
    - ⌥-choosing a script opens the script file for editing instead of running it.
    - A trailing "Open Scripts Folder" item ends the menu.

Further details that are easy to miss:

- **Syntax popup.** The Syntax popup in the toolbar has a "Recently Used" section (max 6) above the full list. A deleted syntax still in use appears under a "Deleted" header.
- **Read-only badge.** "Prevent Editing" shows a pencil-slash badge in the status bar and disables every editing command.
- **Warnings pane.** It never opens by itself; alerts offer "Review" or "Change Encoding" buttons that open it. Incompatible characters are highlighted in the editor only while the pane is visible.
- **Theme pairing.** Themes come in light/dark pairs named `X` and `X (Dark)`. The app switches between them with the system appearance unless the user pinned a theme of the other appearance.
- **Zoom.** Zoom (⌘+ / ⌘− / ⌘0) is a per-editor view scale in steps of ×1.1. It does not change the font setting.
- **Settings window.** Changes apply instantly (no OK/Apply). The window's height animates when switching panes, and the last pane is remembered.
- **Undo names.** Every command registers a descriptive undo name, shown in Edit → Undo/Redo.


