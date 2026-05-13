The 14 Universal Patterns

  1. Spatial Consistency & Persistent Panels

  Panels stay in the same place always. Users build spatial memory ("commits are always bottom-left").
  Lazygit shows 6 panels simultaneously; yazi uses 3 fixed miller columns. The layout never rearranges —
   only content changes.

  2. Linked Master-Detail Panes

  Selecting an item in one pane updates an adjacent pane. Dive (layer→file tree), lazygit (commit→diff),
   yazi (file→preview), K9s (resource→details). This is the single most common layout pattern across top
   TUIs.

  3. Layered Discoverability

  Four levels, from passive to active:
  - Persistent hint bar (Zellij): always-visible footer showing current-mode keys
  - On-demand help (lazygit ?): full keybinding list for current context
  - Action menu (lazygit x): popup listing all possible actions
  - Command palette (K9s :): text entry with tab completion

  The best TUIs use 2-3 of these simultaneously.

  4. Vim Keybinding Vocabulary

  j/k up/down, h/l left/right, / search, Esc back, Enter confirm, q quit. Nearly universal. Also:
  mnemonic action keys (c commit, d delete, a add) that match the first letter of the action.

  5. Global Keybinding Consistency

  Lazygit's rule: "No global keybinding should have context-specific overrides." If R means refresh
  globally, no panel can redefine R. Same key = same action everywhere.

  6. Color as Semantic Information

  - Green = success/active/added
  - Red = error/destructive/deleted
  - Yellow = warning/notes/modified
  - Cyan = accent/shortcuts/interactive elements
  - DarkGray = labels/metadata/muted

  The 60-30-10 rule: 60% primary (default text), 30% secondary (borders, labels), 10% accent
  (highlights, active elements). Color should never be the only signal — pair with symbols and position.

  7. Design in Color Layers

  Three tiers, each standing independently:
  1. Monochrome — is it usable?
  2. 16 ANSI colors — is it readable?
  3. True color (24-bit) — is it beautiful?

  btop exemplifies this with braille→block→TTY graceful degradation.

  8. Information Density Without Clutter

  btop shows CPU, memory, disk, network, and processes on one screen using braille-character graphs. The
   key technique: use bold/dim/color to create visual hierarchy so dense information remains scannable.

  9. Undo Over Confirmation

  Lazygit's z undo is its most praised UX feature. Execute immediately → show brief notification → offer
   undo. This is less disruptive than "Are you sure?" dialogs and enables fearless experimentation.
  Reserve confirmation dialogs for truly irreversible actions only.

  10. Three Tiers of Destructive Action Protection

  Matched to severity:
  - Undo (reversible actions): execute immediately, offer undo
  - Confirmation prompt (hard-to-reverse): "Are you sure? This will delete X"
  - Type-to-confirm (irreversible): force user to type the resource name

  11. Instant Feedback (<100ms)

  If no response within 100ms, the app feels broken. For long operations, show progress indicators.
  Async operations show progress without blocking. Animations must never delay user input — if a key is
  pressed during a transition, cancel the animation.

  12. Inline Error Display

  TUIs lack popup dialogs. Display errors inline at the bottom in red, auto-cleared on next interaction.
   Never swallow errors silently (let _ = on Results is an anti-pattern).

  13. Responsive/Adaptive Layouts

  - Define minimum terminal size (80x24)
  - Use constraint-based layouts (percentages, min/max, ratios)
  - Progressive panel hiding at narrow widths
  - Test at 80x24, 120x40, and 200x60

  14. Sensible Defaults, Zero Configuration Required

  Zellij "ships with sensible defaults requiring no configuration file." Users should have a good first
  experience without touching config. Power users can customize later.

  ---
