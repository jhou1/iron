# Quotes Spec

Browse, inspect, add, edit, delete, and restore personal quotes.

## Layout and preview

The bordered list uses compact one-row entries. Long quote text is truncated by terminal display width only in the list; the source text is never changed. The selected quote also opens in a floating preview that wraps the complete text and shows `↑`, `↓`, or `↑↓` when more content exists.

Quotes composes its presentation from three shared components with explicit ownership boundaries:

- `ScreenFrame` uses its generic one-pass sections API to reserve the list, spacer, optional action area, status row, and wrapped shortcut rows together.
- `ScrollableListPanel` owns the bordered list, its safe outer and inner `ListPanelAreas` geometry, overflow suffix, viewport reveal and clamp, selected-row highlight, and empty-list rendering. Quotes derives preview measurement and placement only from the returned safe inner geometry.
- `ActionPanel` owns bounded rendering of the Add, Edit, and ConfirmDelete action content.

Quotes still owns quote row formatting, domain modes, input and database mutations, floating-preview measurement and placement, and Detail focus/scroll policy. It integrates that preview through the existing shared `render_detail_panel` helper; Batch 3B-2 does not introduce a generic popup abstraction.

Press `Enter` in Browse to focus the complete preview. In Detail mode, `j/k` or the arrow keys scroll by visual row, and `Esc` returns to Browse. The focused border uses the shared accent color.

Batch 3B-2 changes only this presentation composition. Floating-preview behavior, Browse and Detail behavior, active-mode shortcuts and status/error retention, delete/undo ordering, mutation failure retention, and re-entry behavior remain unchanged.

## Modes

| Mode | Trigger | Behavior |
|------|---------|----------|
| Browse | default | Navigate the compact list and preview the selection |
| Detail | `Enter` | Focus and scroll the complete selected quote |
| Add | `a` | Enter a new quote |
| Edit | `e` | Edit the selected quote |
| ConfirmDelete | `d` | Confirm deletion with `y`; any other key cancels |

Add and Edit use the shared bounded single-line input with Unicode-safe Emacs cursor keys. Empty input, database errors, or refresh errors keep the current mode, text, and cursor available. Input clears and Browse resumes only after the mutation and refresh both succeed.

## Delete and undo

Successful deletion stores one quote for `u` undo. Failed delete or restore operations show an error and retain the relevant confirmation or undo state. Selection is restored by quote ID after refresh when that quote still exists.

## Shortcuts

The shortcut row always describes the active mode: Browse lists navigation, detail, add, edit, delete, optional undo, and back; Detail lists scrolling and back; Add/Edit list confirm and cancel; ConfirmDelete lists yes and cancel.

## Re-entry

Opening Quotes reloads current quotes before the route changes. It restores the selected quote by stable ID when possible, preserves bounded list/detail viewports, and keeps Detail focus only when a quote remains. Add, Edit, and ConfirmDelete return to Browse; input, editing identity, old status, and the cross-page undo payload clear.

In-place mutation failures still retain their active mode and input. If cross-page preparation fails, navigation stays on the source screen, the cached Quotes screen is unchanged, and the source shows the error.
