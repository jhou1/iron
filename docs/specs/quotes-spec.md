# Quotes Spec

Browse, inspect, add, edit, delete, and restore personal quotes.

## Layout and preview

The bordered list uses compact one-row entries. Long quote text is truncated by terminal display width only in the list; the source text is never changed. The selected quote also opens in a floating preview that wraps the complete text and shows `↑`, `↓`, or `↑↓` when more content exists.

Press `Enter` in Browse to focus the complete preview. In Detail mode, `j/k` or the arrow keys scroll by visual row, and `Esc` returns to Browse. The focused border uses the shared accent color.

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
